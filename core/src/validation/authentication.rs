use crate::models::authentication::{
    AuthenticationPolicyDefinition, BasicAuthenticationSchemeDefinition,
    BearerAuthenticationSchemeDefinition, DigestAuthenticationSchemeDefinition,
    OAuth2AuthenticationClientDefinition, OAuth2AuthenticationEndpointsDefinition,
    OAuth2AuthenticationRequestDefinition, OAuth2AuthenticationSchemeDefinition,
    OpenIDConnectSchemeDefinition,
};
use super::{ValidationResult, ValidationRule};
use super::enum_validators::{
    validate_oauth2_client_auth_method, validate_oauth2_grant_type,
    validate_oauth2_request_encoding,
};
use super::one_of_validators::validate_auth_policy_one_of;

/// Validates a Basic authentication scheme for mutual exclusivity
/// Basic auth must have either `use` (secret reference) OR username/password, not both
pub fn validate_basic_auth(
    basic: &BasicAuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = basic.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_credentials = basic.username.as_ref().is_some_and(|s| !s.is_empty())
        || basic.password.as_ref().is_some_and(|s| !s.is_empty());
    if has_use && has_credentials {
        result.add_error(
            &format!("{}.basic", prefix),
            ValidationRule::MutualExclusion,
            "basic auth: 'use' and username/password are mutually exclusive",
        );
    }
}

/// Validates a Bearer authentication scheme for mutual exclusivity
/// Bearer auth must have either `use` (secret reference) OR token, not both
pub fn validate_bearer_auth(
    bearer: &BearerAuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = bearer.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_token = bearer.token.as_ref().is_some_and(|s| !s.is_empty());
    if has_use && has_token {
        result.add_error(
            &format!("{}.bearer", prefix),
            ValidationRule::MutualExclusion,
            "bearer auth: 'use' and token are mutually exclusive",
        );
    }
}

/// Validates a Digest authentication scheme for mutual exclusivity
/// Digest auth must have either `use` (secret reference) OR username/password, not both
pub fn validate_digest_auth(
    digest: &DigestAuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = digest.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_credentials = digest.username.as_ref().is_some_and(|s| !s.is_empty())
        || digest.password.as_ref().is_some_and(|s| !s.is_empty());
    if has_use && has_credentials {
        result.add_error(
            &format!("{}.digest", prefix),
            ValidationRule::MutualExclusion,
            "digest auth: 'use' and username/password are mutually exclusive",
        );
    }
}

/// Shared validation for OAuth2-like authentication schemes (OAuth2 and OIDC)
fn validate_oauth2_like_auth(
    scheme_name: &str,
    use_: &Option<String>,
    authority: &Option<String>,
    grant: &Option<String>,
    client: &Option<OAuth2AuthenticationClientDefinition>,
    endpoints: &Option<OAuth2AuthenticationEndpointsDefinition>,
    scopes: &Option<Vec<String>>,
    audiences: &Option<Vec<String>>,
    issuers: &Option<Vec<String>>,
    request: &Option<OAuth2AuthenticationRequestDefinition>,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_properties = authority.as_ref().is_some_and(|s| !s.is_empty())
        || grant.as_ref().is_some_and(|s| !s.is_empty())
        || client.is_some()
        || endpoints.is_some()
        || scopes.is_some()
        || audiences.is_some()
        || issuers.is_some();

    if has_use && has_properties {
        result.add_error(
            &format!("{}.{}", prefix, scheme_name),
            ValidationRule::MutualExclusion,
            &format!("{} auth: 'use' and inline properties are mutually exclusive", scheme_name),
        );
    }
    if !has_use && !has_properties {
        result.add_error(
            &format!("{}.{}", prefix, scheme_name),
            ValidationRule::Required,
            &format!("{} auth: either 'use' or inline properties must be set", scheme_name),
        );
    }
    if let Some(ref grant) = grant {
        validate_oauth2_grant_type(grant, &format!("{}.{}", prefix, scheme_name), result);
    }
    if let Some(ref client) = client {
        if let Some(ref auth_method) = client.authentication {
            validate_oauth2_client_auth_method(auth_method, &format!("{}.{}", prefix, scheme_name), result);
        }
    }
    if let Some(ref request) = request {
        validate_oauth2_request_encoding(&request.encoding, &format!("{}.{}", prefix, scheme_name), result);
    }
}

/// Validates an OAuth2 authentication scheme for mutual exclusivity
pub fn validate_oauth2_auth(
    oauth2: &OAuth2AuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_oauth2_like_auth(
        "oauth2",
        &oauth2.use_,
        &oauth2.authority,
        &oauth2.grant,
        &oauth2.client,
        &oauth2.endpoints,
        &oauth2.scopes,
        &oauth2.audiences,
        &oauth2.issuers,
        &oauth2.request,
        prefix,
        result,
    );
}

/// Validates an OIDC authentication scheme for mutual exclusivity
pub fn validate_oidc_auth(
    oidc: &OpenIDConnectSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_oauth2_like_auth(
        "oidc",
        &oidc.use_,
        &oidc.authority,
        &oidc.grant,
        &oidc.client,
        &None, // OIDC doesn't have endpoints
        &oidc.scopes,
        &oidc.audiences,
        &oidc.issuers,
        &oidc.request,
        prefix,
        result,
    );
}

/// Validates an authentication policy definition
pub fn validate_auth_policy(
    policy: &AuthenticationPolicyDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // Enforce oneOf constraint: only one authentication scheme should be set
    validate_auth_policy_one_of(policy, prefix, result);

    if let Some(ref basic) = policy.basic {
        validate_basic_auth(basic, prefix, result);
    }
    if let Some(ref bearer) = policy.bearer {
        validate_bearer_auth(bearer, prefix, result);
    }
    if let Some(ref digest) = policy.digest {
        validate_digest_auth(digest, prefix, result);
    }
    if let Some(ref oauth2) = policy.oauth2 {
        validate_oauth2_auth(oauth2, prefix, result);
    }
    if let Some(ref oidc) = policy.oidc {
        validate_oidc_auth(oidc, prefix, result);
    }
}
