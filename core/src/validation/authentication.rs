use crate::models::authentication::*;
use super::{ValidationResult, ValidationRule};
use super::enum_validators::{
    validate_oauth2_grant_type, validate_oauth2_client_auth_method, validate_oauth2_request_encoding,
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

/// Validates an OAuth2 authentication scheme for mutual exclusivity
/// OAuth2 must have either `use` (secret reference) OR properties (authority/grant/client/etc.), not both
/// At least one of `use` or properties must be set
pub fn validate_oauth2_auth(
    oauth2: &OAuth2AuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = oauth2.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_properties = oauth2.authority.as_ref().is_some_and(|s| !s.is_empty())
        || oauth2.grant.as_ref().is_some_and(|s| !s.is_empty())
        || oauth2.client.is_some()
        || oauth2.endpoints.is_some()
        || oauth2.scopes.is_some()
        || oauth2.audiences.is_some()
        || oauth2.issuers.is_some();

    if has_use && has_properties {
        result.add_error(
            &format!("{}.oauth2", prefix),
            ValidationRule::MutualExclusion,
            "oauth2 auth: 'use' and inline properties are mutually exclusive",
        );
    }
    if !has_use && !has_properties {
        result.add_error(
            &format!("{}.oauth2", prefix),
            ValidationRule::Required,
            "oauth2 auth: either 'use' or inline properties must be set",
        );
    }
    // Validate grant type if set (matches Go SDK's oneof validation)
    if let Some(ref grant) = oauth2.grant {
        validate_oauth2_grant_type(grant, &format!("{}.oauth2", prefix), result);
    }
    // Validate client authentication method if set
    if let Some(ref client) = oauth2.client {
        if let Some(ref auth_method) = client.authentication {
            validate_oauth2_client_auth_method(auth_method, &format!("{}.oauth2", prefix), result);
        }
    }
    // Validate request encoding if set
    if let Some(ref request) = oauth2.request {
        validate_oauth2_request_encoding(&request.encoding, &format!("{}.oauth2", prefix), result);
    }
}

/// Validates an OIDC authentication scheme for mutual exclusivity
/// OIDC must have either `use` (secret reference) OR properties (authority/grant/client/etc.), not both
/// At least one of `use` or properties must be set
pub fn validate_oidc_auth(
    oidc: &OpenIDConnectSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = oidc.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_properties = oidc.authority.as_ref().is_some_and(|s| !s.is_empty())
        || oidc.grant.as_ref().is_some_and(|s| !s.is_empty())
        || oidc.client.is_some()
        || oidc.scopes.is_some()
        || oidc.audiences.is_some()
        || oidc.issuers.is_some();

    if has_use && has_properties {
        result.add_error(
            &format!("{}.oidc", prefix),
            ValidationRule::MutualExclusion,
            "oidc auth: 'use' and inline properties are mutually exclusive",
        );
    }
    if !has_use && !has_properties {
        result.add_error(
            &format!("{}.oidc", prefix),
            ValidationRule::Required,
            "oidc auth: either 'use' or inline properties must be set",
        );
    }
    // Validate grant type if set (matches Go SDK's oneof validation)
    if let Some(ref grant) = oidc.grant {
        validate_oauth2_grant_type(grant, &format!("{}.oidc", prefix), result);
    }
    // Validate client authentication method if set
    if let Some(ref client) = oidc.client {
        if let Some(ref auth_method) = client.authentication {
            validate_oauth2_client_auth_method(auth_method, &format!("{}.oidc", prefix), result);
        }
    }
    // Validate request encoding if set
    if let Some(ref request) = oidc.request {
        validate_oauth2_request_encoding(&request.encoding, &format!("{}.oidc", prefix), result);
    }
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
