use crate::error::{WorkflowError, WorkflowResult};
use crate::expression::evaluate_expression_str;
use serde_json::Value;
use serverless_workflow_core::models::authentication::ReferenceableAuthenticationPolicy;

/// Applies authentication to an HTTP request builder
/// Returns (request_builder, optional_authorization) where authorization is (scheme, parameter)
pub(crate) async fn apply_authentication(
    mut builder: reqwest::RequestBuilder,
    policy: &ReferenceableAuthenticationPolicy,
    auth_definitions: Option<&std::collections::HashMap<String, ReferenceableAuthenticationPolicy>>,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<(reqwest::RequestBuilder, Option<(String, String)>)> {
    // Resolve the policy: if it's a reference, look it up in workflow.use_.authentications
    let resolved_policy = match policy {
        ReferenceableAuthenticationPolicy::Policy(def) => def,
        ReferenceableAuthenticationPolicy::Reference(ref_ref) => {
            match auth_definitions {
                Some(defs) => {
                    match defs.get(&ref_ref.use_) {
                        Some(ReferenceableAuthenticationPolicy::Policy(def)) => def,
                        Some(ReferenceableAuthenticationPolicy::Reference(nested)) => {
                            // Prevent deep recursion: only resolve one level of reference
                            return Err(WorkflowError::validation(
                                format!(
                                    "nested authentication reference '{}' is not supported",
                                    nested.use_
                                ),
                                task_name,
                            ));
                        }
                        None => {
                            return Err(WorkflowError::validation(
                            format!("authentication reference '{}' not found in use.authentications", ref_ref.use_),
                            task_name,
                        ));
                        }
                    }
                }
                None => {
                    return Err(WorkflowError::validation(
                        format!(
                            "authentication reference '{}' but no use.authentications defined",
                            ref_ref.use_
                        ),
                        task_name,
                    ));
                }
            }
        }
    };

    // Apply basic authentication
    let mut authorization: Option<(String, String)> = None;
    if let Some(ref basic) = resolved_policy.basic {
        let (auth_scheme, creds) = apply_credentials_auth(
            "Basic",
            &basic.username,
            &basic.password,
            basic.use_.as_deref(),
            input,
            vars,
            task_name,
        )
        .await?;
        if let Some((username, password)) = creds {
            let parameter = format!("{}:{}", username, password);
            authorization = Some((auth_scheme, parameter));
            builder = builder.basic_auth(username, Some(password));
        }
    }

    // Apply bearer authentication
    if let Some(ref bearer) = resolved_policy.bearer {
        if let Some(ref token_expr) = bearer.token {
            let token = evaluate_expression_str(token_expr, input, vars, task_name)?;
            authorization = Some(("Bearer".to_string(), token.clone()));
            builder = builder.bearer_auth(token);
        } else if let Some(ref secret_name) = bearer.use_ {
            // Look up token from $secret.<secretName>
            let token = lookup_secret_token(secret_name, vars, task_name)?;
            authorization = Some(("Bearer".to_string(), token.clone()));
            builder = builder.bearer_auth(&token);
        }
    }

    // Apply digest authentication
    if let Some(ref digest) = resolved_policy.digest {
        let (auth_scheme, creds) = apply_credentials_auth(
            "Digest",
            &digest.username,
            &digest.password,
            digest.use_.as_deref(),
            input,
            vars,
            task_name,
        )
        .await?;
        if let Some((username, password)) = creds {
            // Digest auth requires a two-step flow (pre-flight + retry with digest header).
            // We apply basic_auth as a fallback here — the actual digest flow is handled
            // in the response processing code when a 401 with WWW-Authenticate: Digest is received.
            let parameter = format!("{}:{}", username, password);
            authorization = Some((auth_scheme, parameter));
            builder = builder.basic_auth(username, Some(password));
        }
    }

    // Apply OAuth2 authentication — fetch access token from token endpoint
    if let Some(ref oauth2) = resolved_policy.oauth2 {
        let access_token = fetch_oauth2_token(oauth2, input, vars, task_name).await?;
        authorization = Some(("Bearer".to_string(), access_token.clone()));
        builder = builder.bearer_auth(&access_token);
    }

    // Apply OIDC authentication — same flow as OAuth2 (fetch token, use as Bearer)
    if let Some(ref oidc) = resolved_policy.oidc {
        let access_token = fetch_oidc_token(oidc, input, vars, task_name).await?;
        authorization = Some(("Bearer".to_string(), access_token.clone()));
        builder = builder.bearer_auth(&access_token);
    }

    Ok((builder, authorization))
}

/// Applies credential-based authentication (Basic or Digest) by extracting
/// username/password either from inline expressions or a secret reference.
/// Returns the auth scheme name and optional (username, password) credentials.
async fn apply_credentials_auth(
    scheme: &str,
    username_expr: &Option<String>,
    password_expr: &Option<String>,
    secret_ref: Option<&str>,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<(String, Option<(String, String)>)> {
    if let Some(ref username) = username_expr {
        let username_val = evaluate_expression_str(username, input, vars, task_name)?;
        let password_val = password_expr
            .as_deref()
            .map(|p| evaluate_expression_str(p, input, vars, task_name))
            .transpose()?
            .unwrap_or_default();
        Ok((scheme.to_string(), Some((username_val, password_val))))
    } else if let Some(secret_name) = secret_ref {
        let (username_val, password_val) = lookup_secret_credentials(secret_name, vars, task_name)?;
        Ok((scheme.to_string(), Some((username_val, password_val))))
    } else {
        Ok((scheme.to_string(), None))
    }
}

/// Looks up a secret object from $secret.<secretName>
fn lookup_secret<'a>(
    secret_name: &str,
    vars: &'a std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<&'a Value> {
    vars.get("$secret")
        .and_then(|s| s.get(secret_name))
        .ok_or_else(|| {
            WorkflowError::validation(
                format!("secret '{}' not found for authentication", secret_name),
                task_name,
            )
        })
}

/// Looks up username and password from $secret.<secretName> for basic/digest auth
/// The secret object should contain "username" and "password" fields
fn lookup_secret_credentials(
    secret_name: &str,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<(String, String)> {
    let secret = lookup_secret(secret_name, vars, task_name)?;

    let username = secret
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WorkflowError::validation(
                format!("secret '{}' missing 'username' field", secret_name),
                task_name,
            )
        })?
        .to_string();

    let password = secret
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok((username, password))
}

/// Looks up token from $secret.<secretName> for bearer auth
/// The secret object should contain a "token" field
fn lookup_secret_token(
    secret_name: &str,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    let secret = lookup_secret(secret_name, vars, task_name)?;

    secret
        .get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            WorkflowError::validation(
                format!("secret '{}' missing 'token' field", secret_name),
                task_name,
            )
        })
}

/// Shared parameters for token endpoint requests (used by both OAuth2 and OIDC)
struct TokenRequestParams {
    token_url: String,
    grant_type: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    encoding: String,
    scopes: String,
    /// Grant-type-specific key-value pairs (username/password, subject_token, etc.)
    grant_params: Vec<(String, String)>,
    client_auth_method: String,
    assertion: Option<String>,
    issuers: Option<Vec<String>>,
    protocol_name: &'static str,
}

/// Common token fetching logic shared by OAuth2 and OIDC.
/// Sends a token request to the endpoint and returns the access_token.
async fn fetch_access_token(params: TokenRequestParams, task_name: &str) -> WorkflowResult<String> {
    let protocol = params.protocol_name;

    let mut form_params = vec![("grant_type".to_string(), params.grant_type.clone())];
    form_params.extend(params.grant_params);

    if !params.scopes.is_empty() {
        form_params.push(("scope".to_string(), params.scopes));
    }

    let client = reqwest::Client::new();
    let mut request_builder = client.post(&params.token_url);

    // Client authentication
    match params.client_auth_method.as_str() {
        "client_secret_basic" | "none" => {
            if let (Some(id), Some(secret)) = (&params.client_id, &params.client_secret) {
                request_builder = request_builder.basic_auth(id, Some(secret));
            } else if let Some(id) = &params.client_id {
                request_builder = request_builder.basic_auth(id, Some(""));
            }
        }
        _ => {
            // client_secret_post (default): client_id/client_secret in request body
            if let Some(id) = &params.client_id {
                form_params.push(("client_id".to_string(), id.clone()));
            }
            if let Some(secret) = &params.client_secret {
                form_params.push(("client_secret".to_string(), secret.clone()));
            }
        }
    }

    // Handle assertion for JWT bearer
    if let Some(assertion) = &params.assertion {
        form_params.push(("assertion".to_string(), assertion.clone()));
    }

    // Send request with appropriate encoding
    let response = if params.encoding.contains("json") {
        let body: Value = serde_json::json!(form_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<std::collections::HashMap<&str, &str>>());
        request_builder.json(&body)
    } else {
        request_builder.form(&form_params)
    }
    .send()
    .await
    .map_err(|e| {
        WorkflowError::communication(
            format!("{} token request failed: {}", protocol, e),
            task_name,
        )
    })?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        return Err(WorkflowError::communication(
            format!(
                "{} token endpoint returned status {}: {}",
                protocol, status, body_text
            ),
            task_name,
        ));
    }

    let token_response: Value = response.json().await.map_err(|e| {
        WorkflowError::communication(
            format!("failed to parse {} token response: {}", protocol, e),
            task_name,
        )
    })?;

    let access_token = token_response
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WorkflowError::communication(
                format!("{} token response missing 'access_token' field", protocol),
                task_name,
            )
        })?;

    // Validate issuer if configured
    if let Some(ref issuers) = params.issuers {
        if let Some(issuer) = token_response.get("iss").and_then(|v| v.as_str()) {
            if !issuers.iter().any(|i| i == issuer) {
                return Err(WorkflowError::communication(
                    format!(
                        "{} token issuer '{}' not in allowed list: {:?}",
                        protocol, issuer, issuers
                    ),
                    task_name,
                ));
            }
        }
    }

    Ok(access_token.to_string())
}

/// Shared fields extracted from OAuth2/OIDC scheme definitions for token requests.
/// Both `OAuth2AuthenticationSchemeDefinition` and `OpenIDConnectSchemeDefinition`
/// share the same structure for these fields, so this struct consolidates extraction.
struct OAuthTokenFields {
    client: Option<
        serverless_workflow_core::models::authentication::OAuth2AuthenticationClientDefinition,
    >,
    grant: Option<String>,
    request: Option<
        serverless_workflow_core::models::authentication::OAuth2AuthenticationRequestDefinition,
    >,
    issuers: Option<Vec<String>>,
    scopes: Option<Vec<String>>,
    username: Option<String>,
    password: Option<String>,
    subject: Option<serverless_workflow_core::models::authentication::OAuth2TokenDefinition>,
    actor: Option<serverless_workflow_core::models::authentication::OAuth2TokenDefinition>,
}

impl OAuthTokenFields {
    fn from_oauth2(
        oauth2: &serverless_workflow_core::models::authentication::OAuth2AuthenticationSchemeDefinition,
    ) -> Self {
        Self {
            client: oauth2.client.clone(),
            grant: oauth2.grant.clone(),
            request: oauth2.request.clone(),
            issuers: oauth2.issuers.clone(),
            scopes: oauth2.scopes.clone(),
            username: oauth2.username.clone(),
            password: oauth2.password.clone(),
            subject: oauth2.subject.clone(),
            actor: oauth2.actor.clone(),
        }
    }

    fn from_oidc(
        oidc: &serverless_workflow_core::models::authentication::OpenIDConnectSchemeDefinition,
    ) -> Self {
        Self {
            client: oidc.client.clone(),
            grant: oidc.grant.clone(),
            request: oidc.request.clone(),
            issuers: oidc.issuers.clone(),
            scopes: oidc.scopes.clone(),
            username: oidc.username.clone(),
            password: oidc.password.clone(),
            subject: oidc.subject.clone(),
            actor: oidc.actor.clone(),
        }
    }
}

/// Builds a TokenRequestParams from shared OAuth token fields.
/// `token_url` is pre-computed (OAuth2 appends endpoint path, OIDC uses authority directly).
/// `protocol_name` is "OAuth2" or "OIDC".
/// `allow_token_exchange` enables the token-exchange grant type (only for OAuth2).
fn build_token_request_params(
    token_url: String,
    fields: OAuthTokenFields,
    protocol_name: &'static str,
    allow_token_exchange: bool,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<TokenRequestParams> {
    let grant_type = fields
        .grant
        .as_deref()
        .unwrap_or("client_credentials")
        .to_string();

    let client_id = fields
        .client
        .as_ref()
        .and_then(|c| c.id.as_deref())
        .map(|id| evaluate_expression_str(id, input, vars, task_name))
        .transpose()?;
    let client_secret = fields
        .client
        .as_ref()
        .and_then(|c| c.secret.as_deref())
        .map(|s| evaluate_expression_str(s, input, vars, task_name))
        .transpose()?;

    let encoding = fields
        .request
        .as_ref()
        .map(|r| r.encoding.as_str())
        .unwrap_or("application/x-www-form-urlencoded")
        .to_string();

    let scopes = fields
        .scopes
        .as_ref()
        .map(|s| s.join(" "))
        .unwrap_or_default();

    // Build grant-type-specific params
    let mut grant_params = Vec::new();
    match grant_type.as_str() {
        "client_credentials" => { /* scope handled in fetch_access_token */ }
        "password" => {
            let username = fields
                .username
                .as_deref()
                .map(|u| evaluate_expression_str(u, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            let password = fields
                .password
                .as_deref()
                .map(|p| evaluate_expression_str(p, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            grant_params.push(("username".to_string(), username));
            grant_params.push(("password".to_string(), password));
        }
        "urn:ietf:params:oauth:grant-type:token-exchange" if allow_token_exchange => {
            if let Some(ref subject) = fields.subject {
                let subject_token =
                    evaluate_expression_str(&subject.token, input, vars, task_name)?;
                grant_params.push(("subject_token".to_string(), subject_token));
                grant_params.push((
                    "subject_token_type".to_string(),
                    subject.type_.as_str().to_string(),
                ));
            }
            if let Some(ref actor) = fields.actor {
                let actor_token = evaluate_expression_str(&actor.token, input, vars, task_name)?;
                grant_params.push(("actor_token".to_string(), actor_token));
                grant_params.push((
                    "actor_token_type".to_string(),
                    actor.type_.as_str().to_string(),
                ));
            }
        }
        _ => {
            return Err(WorkflowError::validation(
                format!("unsupported {} grant type: '{}'", protocol_name, grant_type),
                task_name,
            ));
        }
    }

    let client_auth_method = fields
        .client
        .as_ref()
        .and_then(|c| c.authentication.as_deref())
        .unwrap_or("client_secret_post")
        .to_string();

    let assertion = fields
        .client
        .as_ref()
        .and_then(|c| c.assertion.as_deref())
        .map(|a| evaluate_expression_str(a, input, vars, task_name))
        .transpose()?;

    Ok(TokenRequestParams {
        token_url,
        grant_type,
        client_id,
        client_secret,
        encoding,
        scopes,
        grant_params,
        client_auth_method,
        assertion,
        issuers: fields.issuers,
        protocol_name,
    })
}

/// Fetches an OAuth2 access token from the token endpoint
/// Implements the client_credentials, password, and token-exchange grant types
/// matching Java SDK's JaxRSAccessTokenProvider
async fn fetch_oauth2_token(
    oauth2: &serverless_workflow_core::models::authentication::OAuth2AuthenticationSchemeDefinition,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    // Build the token endpoint URL from authority + endpoints.token path
    let authority = oauth2
        .authority
        .as_deref()
        .map(|a| evaluate_expression_str(a, input, vars, task_name))
        .transpose()?
        .ok_or_else(|| {
            WorkflowError::validation(
                "OAuth2 authentication requires 'authority' to be set".to_string(),
                task_name,
            )
        })?;
    let token_path = oauth2
        .endpoints
        .as_ref()
        .map(|e| e.token.as_str())
        .unwrap_or("/oauth2/token");
    let token_url = format!("{}{}", authority.trim_end_matches('/'), token_path);

    let params = build_token_request_params(
        token_url,
        OAuthTokenFields::from_oauth2(oauth2),
        "OAuth2",
        true,
        input,
        vars,
        task_name,
    )?;
    fetch_access_token(params, task_name).await
}

/// Fetches an OIDC access token — same as OAuth2 but OIDC's authority IS the token endpoint URL
async fn fetch_oidc_token(
    oidc: &serverless_workflow_core::models::authentication::OpenIDConnectSchemeDefinition,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    // For OIDC, the authority is the full token endpoint URL
    let token_url = oidc
        .authority
        .as_deref()
        .map(|a| evaluate_expression_str(a, input, vars, task_name))
        .transpose()?
        .ok_or_else(|| {
            WorkflowError::validation(
                "OIDC authentication requires 'authority' to be set".to_string(),
                task_name,
            )
        })?;

    let params = build_token_request_params(
        token_url,
        OAuthTokenFields::from_oidc(oidc),
        "OIDC",
        false,
        input,
        vars,
        task_name,
    )?;
    fetch_access_token(params, task_name).await
}
