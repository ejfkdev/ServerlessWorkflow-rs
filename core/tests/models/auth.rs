use super::*;

#[test]
fn test_basic_authentication_serialization() {
    // Test BasicAuthenticationSchemeDefinition serialization (similar to Go SDK TestAuthenticationPolicy)
    let basic_auth_json = json!({
        "username": "john",
        "password": "12345"
    });
    let result: Result<BasicAuthenticationSchemeDefinition, _> =
        serde_json::from_value(basic_auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize basic auth: {:?}",
        result.err()
    );
    let basic_auth = result.unwrap();
    assert_eq!(basic_auth.username, Some("john".to_string()));
    assert_eq!(basic_auth.password, Some("12345".to_string()));
}

#[test]
fn test_bearer_authentication_serialization() {
    // Test BearerAuthenticationSchemeDefinition serialization
    let bearer_auth_json = json!({
        "token": "my-bearer-token"
    });
    let result: Result<BearerAuthenticationSchemeDefinition, _> =
        serde_json::from_value(bearer_auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize bearer auth: {:?}",
        result.err()
    );
    let bearer_auth = result.unwrap();
    assert_eq!(bearer_auth.token, Some("my-bearer-token".to_string()));
}

#[test]
fn test_authentication_policy_with_basic() {
    // Test AuthenticationPolicyDefinition with basic auth
    let auth_policy_json = json!({
        "basic": {
            "username": "john",
            "password": "12345"
        }
    });
    let result: Result<AuthenticationPolicyDefinition, _> =
        serde_json::from_value(auth_policy_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize auth policy: {:?}",
        result.err()
    );
    let auth_policy = result.unwrap();
    assert!(auth_policy.basic.is_some());
    let basic = auth_policy.basic.unwrap();
    assert_eq!(basic.username, Some("john".to_string()));
    assert_eq!(basic.password, Some("12345".to_string()));
}

#[test]
fn test_authentication_policy_with_bearer() {
    // Test AuthenticationPolicyDefinition with bearer auth
    let auth_policy_json = json!({
        "bearer": {
            "token": "my-bearer-token"
        }
    });
    let result: Result<AuthenticationPolicyDefinition, _> =
        serde_json::from_value(auth_policy_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize auth policy: {:?}",
        result.err()
    );
    let auth_policy = result.unwrap();
    assert!(auth_policy.bearer.is_some());
    let bearer = auth_policy.bearer.unwrap();
    assert_eq!(bearer.token, Some("my-bearer-token".to_string()));
}

#[test]
fn test_authentication_policy_roundtrip() {
    // Test AuthenticationPolicyDefinition roundtrip serialization
    let auth_policy = AuthenticationPolicyDefinition {
        use_: Some("my-auth".to_string()),
        basic: Some(BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("john".to_string()),
            password: Some("12345".to_string()),
        }),
        bearer: None,
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };

    let json_str = serde_json::to_string(&auth_policy).expect("Failed to serialize auth policy");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.basic.is_some());
    assert_eq!(
        deserialized.basic.unwrap().username,
        Some("john".to_string())
    );
}

#[test]
fn test_digest_authentication_serialization() {
    // Test DigestAuthenticationSchemeDefinition serialization
    let digest_auth_json = json!({
        "username": "digestUser",
        "password": "digestPass"
    });
    let result: Result<DigestAuthenticationSchemeDefinition, _> =
        serde_json::from_value(digest_auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize digest auth: {:?}",
        result.err()
    );
    let digest_auth = result.unwrap();
    assert_eq!(digest_auth.username, Some("digestUser".to_string()));
    assert_eq!(digest_auth.password, Some("digestPass".to_string()));
}

#[test]
fn test_bearer_authentication_roundtrip() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, BearerAuthenticationSchemeDefinition,
    };
    let auth = AuthenticationPolicyDefinition {
        bearer: Some(BearerAuthenticationSchemeDefinition {
            token: Some("my-token".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.bearer.is_some());
}

#[test]
fn test_digest_authentication_roundtrip() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, DigestAuthenticationSchemeDefinition,
    };
    let auth = AuthenticationPolicyDefinition {
        digest: Some(DigestAuthenticationSchemeDefinition {
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.digest.is_some());
}

#[test]
fn test_bearer_authentication_with_uri() {
    // Test Bearer authentication with URI format
    let bearer_json = json!({
        "token": "${ .token }"
    });

    let result: Result<
        serverless_workflow_core::models::authentication::BearerAuthenticationSchemeDefinition,
        _,
    > = serde_json::from_value(bearer_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize bearer auth: {:?}",
        result.err()
    );
    if let Ok(bearer) = result {
        assert!(bearer.token.is_some());
        if let Some(token) = bearer.token {
            assert!(token.contains(".token"));
        }
    }
}

#[test]
fn test_authentication_reusable() {
    // Test reusable authentication definitions
    let auth_json = json!({
        "name": "myAuth",
        "scheme": {
            "basic": {
                "username": "admin",
                "password": "admin"
            }
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::AuthenticationPolicyDefinition,
        _,
    > = serde_json::from_value(auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize reusable auth: {:?}",
        result.err()
    );
}

#[test]
fn test_oidc_authentication() {
    // Test OIDC authentication from specification
    let oidc_json = json!({
        "oidc": {
            "authority": "https://authority.com",
            "client": {
                "id": "client-id",
                "secret": "client-secret"
            },
            "scopes": ["openid", "profile"]
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::OpenIDConnectSchemeDefinition,
        _,
    > = serde_json::from_value(oidc_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize OIDC auth: {:?}",
        result.err()
    );
}

#[test]
fn test_digest_authentication() {
    // Test Digest authentication
    let digest_json = json!({
        "digest": {
            "username": "user",
            "password": "pass"
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::DigestAuthenticationSchemeDefinition,
        _,
    > = serde_json::from_value(digest_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize digest auth: {:?}",
        result.err()
    );
}
