use super::*;
use crate::models::authentication::*;
use crate::models::call::*;
use crate::models::duration::{Duration, OneOfDurationOrIso8601Expression};
use crate::models::map::Map;
use crate::models::resource::ExternalResourceDefinition;
use crate::models::retry::*;
use crate::models::schema::SchemaDefinition;
use crate::models::task::*;
use crate::models::workflow::*;
use std::collections::HashMap;

#[test]
fn test_valid_semver() {
    // Valid cases
    assert!(is_valid_semver("1.0.0"));
    assert!(is_valid_semver("0.1.0"));
    assert!(is_valid_semver("1.2.3"));
    assert!(is_valid_semver("1.0.0-alpha"));
    assert!(is_valid_semver("1.0.0-alpha.1"));
    assert!(is_valid_semver("1.0.0+build.123"));
    assert!(is_valid_semver("1.2.3-beta.1+build.123"));
    // Invalid cases
    assert!(!is_valid_semver(""));
    assert!(!is_valid_semver("1"));
    assert!(!is_valid_semver("1.0"));
    assert!(!is_valid_semver("v1.0.0"));
    assert!(!is_valid_semver("v1.2.3"));
}

#[test]
fn test_valid_hostname() {
    // Valid cases
    assert!(is_valid_hostname("example"));
    assert!(is_valid_hostname("example.com"));
    assert!(is_valid_hostname("my-host"));
    assert!(is_valid_hostname("my-hostname"));
    assert!(is_valid_hostname("subdomain.example.com"));
    assert!(is_valid_hostname("default"));
    // Invalid cases
    assert!(!is_valid_hostname(""));
    assert!(!is_valid_hostname("-invalid"));
    assert!(!is_valid_hostname("invalid-"));
    assert!(!is_valid_hostname("127.0.0.1"));
    assert!(!is_valid_hostname("example.com."));
    assert!(!is_valid_hostname("example..com"));
    assert!(!is_valid_hostname("example.com-"));
}

#[test]
fn test_valid_iso8601_duration() {
    // Valid cases
    assert!(crate::models::duration::is_iso8601_duration_valid("PT5S"));
    assert!(crate::models::duration::is_iso8601_duration_valid("PT10M"));
    assert!(crate::models::duration::is_iso8601_duration_valid("PT1H"));
    assert!(crate::models::duration::is_iso8601_duration_valid("P1D"));
    assert!(crate::models::duration::is_iso8601_duration_valid(
        "P1DT12H"
    ));
    assert!(crate::models::duration::is_iso8601_duration_valid(
        "P1DT12H30M"
    ));
    assert!(crate::models::duration::is_iso8601_duration_valid(
        "PT1H30M"
    ));
    assert!(crate::models::duration::is_iso8601_duration_valid(
        "PT250MS"
    ));
    assert!(crate::models::duration::is_iso8601_duration_valid(
        "P3DT4H5M6S250MS"
    ));
    // Invalid cases
    assert!(!crate::models::duration::is_iso8601_duration_valid(""));
    assert!(!crate::models::duration::is_iso8601_duration_valid("5S"));
    assert!(!crate::models::duration::is_iso8601_duration_valid("P1Y"));
    assert!(!crate::models::duration::is_iso8601_duration_valid(
        "P1Y2M3D"
    ));
    assert!(!crate::models::duration::is_iso8601_duration_valid("P1W"));
    assert!(!crate::models::duration::is_iso8601_duration_valid(
        "P1Y2M3D4H"
    ));
    assert!(!crate::models::duration::is_iso8601_duration_valid(
        "P1Y2M3D4H5M6S"
    ));
    assert!(!crate::models::duration::is_iso8601_duration_valid("P"));
    assert!(!crate::models::duration::is_iso8601_duration_valid("P1DT"));
    assert!(!crate::models::duration::is_iso8601_duration_valid("1Y"));
    assert!(!crate::models::duration::is_iso8601_duration_valid(
        "P1DT2H3M4S5MS7"
    )); // trailing garbage
}

#[test]
fn test_validate_workflow_valid() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "test-workflow",
        "1.0.0",
        None,
        None,
        None,
    ));
    workflow.do_.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            crate::models::call::CallFunctionDefinition {
                call: "myFunction".to_string(),
                with: None,
                common: TaskDefinitionFields::new(),
            },
        ))),
    );

    let result = validate_workflow(&workflow);
    assert!(
        result.is_valid(),
        "Expected valid workflow, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_workflow_missing_name() {
    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "", "1.0.0", None, None, None,
    ));
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|e| e.field == "document.name"));
}

#[test]
fn test_validate_workflow_invalid_semver() {
    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata {
        dsl: "not-semver".to_string(),
        namespace: "default".to_string(),
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        title: None,
        summary: None,
        tags: None,
        metadata: None,
    });
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|e| e.field == "document.dsl"));
}

#[test]
fn test_validation_result_merge() {
    let mut result1 = ValidationResult::new();
    result1.add_error("field1", ValidationRule::Required, "field1 is required");

    let mut result2 = ValidationResult::new();
    result2.add_error("field2", ValidationRule::Semver, "field2 is not semver");

    result1.merge_with_prefix("parent", result2);
    assert_eq!(result1.errors.len(), 2);
    assert_eq!(result1.errors[1].field, "parent.field2");
}

// Auth policy mutual exclusion tests (matching Go SDK's validator.go)

#[test]
fn test_validate_basic_auth_use_only() {
    let basic = BasicAuthenticationSchemeDefinition {
        use_: Some("mySecret".to_string()),
        username: None,
        password: None,
    };
    let mut result = ValidationResult::new();
    validate_basic_auth(&basic, "auth", &mut result);
    assert!(result.is_valid(), "use-only basic auth should be valid");
}

#[test]
fn test_validate_basic_auth_credentials_only() {
    let basic = BasicAuthenticationSchemeDefinition {
        use_: None,
        username: Some("john".to_string()),
        password: Some("secret".to_string()),
    };
    let mut result = ValidationResult::new();
    validate_basic_auth(&basic, "auth", &mut result);
    assert!(
        result.is_valid(),
        "credentials-only basic auth should be valid"
    );
}

#[test]
fn test_validate_basic_auth_mutual_exclusion() {
    let basic = BasicAuthenticationSchemeDefinition {
        use_: Some("mySecret".to_string()),
        username: Some("john".to_string()),
        password: Some("secret".to_string()),
    };
    let mut result = ValidationResult::new();
    validate_basic_auth(&basic, "auth", &mut result);
    assert!(!result.is_valid(), "use + credentials should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_bearer_auth_use_only() {
    let bearer = BearerAuthenticationSchemeDefinition {
        use_: Some("bearerSecret".to_string()),
        token: None,
    };
    let mut result = ValidationResult::new();
    validate_bearer_auth(&bearer, "auth", &mut result);
    assert!(result.is_valid(), "use-only bearer auth should be valid");
}

#[test]
fn test_validate_bearer_auth_token_only() {
    let bearer = BearerAuthenticationSchemeDefinition {
        use_: None,
        token: Some("mytoken123".to_string()),
    };
    let mut result = ValidationResult::new();
    validate_bearer_auth(&bearer, "auth", &mut result);
    assert!(result.is_valid(), "token-only bearer auth should be valid");
}

#[test]
fn test_validate_bearer_auth_mutual_exclusion() {
    let bearer = BearerAuthenticationSchemeDefinition {
        use_: Some("bearerSecret".to_string()),
        token: Some("mytoken123".to_string()),
    };
    let mut result = ValidationResult::new();
    validate_bearer_auth(&bearer, "auth", &mut result);
    assert!(!result.is_valid(), "use + token should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_digest_auth_use_only() {
    let digest = DigestAuthenticationSchemeDefinition {
        use_: Some("digestSecret".to_string()),
        username: None,
        password: None,
    };
    let mut result = ValidationResult::new();
    validate_digest_auth(&digest, "auth", &mut result);
    assert!(result.is_valid(), "use-only digest auth should be valid");
}

#[test]
fn test_validate_digest_auth_credentials_only() {
    let digest = DigestAuthenticationSchemeDefinition {
        use_: None,
        username: Some("digestUser".to_string()),
        password: Some("digestPass".to_string()),
    };
    let mut result = ValidationResult::new();
    validate_digest_auth(&digest, "auth", &mut result);
    assert!(
        result.is_valid(),
        "credentials-only digest auth should be valid"
    );
}

#[test]
fn test_validate_digest_auth_mutual_exclusion() {
    let digest = DigestAuthenticationSchemeDefinition {
        use_: Some("digestSecret".to_string()),
        username: Some("digestUser".to_string()),
        password: Some("digestPass".to_string()),
    };
    let mut result = ValidationResult::new();
    validate_digest_auth(&digest, "auth", &mut result);
    assert!(!result.is_valid(), "use + credentials should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_oauth2_auth_use_only() {
    let oauth2 = OAuth2AuthenticationSchemeDefinition {
        use_: Some("oauth2Secret".to_string()),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oauth2_auth(&oauth2, "auth", &mut result);
    assert!(result.is_valid(), "use-only oauth2 should be valid");
}

#[test]
fn test_validate_oauth2_auth_properties_only() {
    let oauth2 = OAuth2AuthenticationSchemeDefinition {
        use_: None,
        authority: Some("https://auth.example.com".to_string()),
        grant: Some("client_credentials".to_string()),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oauth2_auth(&oauth2, "auth", &mut result);
    assert!(result.is_valid(), "properties-only oauth2 should be valid");
}

#[test]
fn test_validate_oauth2_auth_mutual_exclusion() {
    let oauth2 = OAuth2AuthenticationSchemeDefinition {
        use_: Some("oauth2Secret".to_string()),
        authority: Some("https://auth.example.com".to_string()),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oauth2_auth(&oauth2, "auth", &mut result);
    assert!(!result.is_valid(), "use + properties should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_oauth2_auth_neither_set() {
    let oauth2 = OAuth2AuthenticationSchemeDefinition {
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oauth2_auth(&oauth2, "auth", &mut result);
    assert!(!result.is_valid(), "empty oauth2 should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::Required));
}

#[test]
fn test_validate_auth_policy_valid() {
    let policy = AuthenticationPolicyDefinition {
        use_: None,
        basic: Some(BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
        }),
        bearer: None,
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };
    let mut result = ValidationResult::new();
    validate_auth_policy(&policy, "auth", &mut result);
    assert!(result.is_valid(), "valid auth policy should pass");
}

#[test]
fn test_validate_auth_policy_multiple_violations() {
    let policy = AuthenticationPolicyDefinition {
        use_: None,
        basic: Some(BasicAuthenticationSchemeDefinition {
            use_: Some("secret".to_string()),
            username: Some("admin".to_string()),
            password: None,
        }),
        bearer: Some(BearerAuthenticationSchemeDefinition {
            use_: Some("bearerSecret".to_string()),
            token: Some("token".to_string()),
        }),
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };
    let mut result = ValidationResult::new();
    validate_auth_policy(&policy, "auth", &mut result);
    assert!(!result.is_valid());
    // 3 errors: 1 oneOf (multiple schemes) + 2 mutual exclusion (basic use+creds, bearer use+token)
    assert_eq!(
        result.errors.len(),
        3,
        "Should have 3 errors (1 oneOf + 2 mutual exclusion), got: {:?}",
        result.errors
    );
}

// Pull policy validation tests (matching Go SDK's oneof=ifNotPresent always never)

#[test]
fn test_validate_pull_policy_valid() {
    for policy in &["ifNotPresent", "always", "never"] {
        let mut result = ValidationResult::new();
        validate_pull_policy(policy, "container", &mut result);
        assert!(result.is_valid(), "'{}' should be valid pullPolicy", policy);
    }
}

#[test]
fn test_validate_pull_policy_invalid() {
    let mut result = ValidationResult::new();
    validate_pull_policy("invalid", "container", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].message.contains("invalid"));
}

// Container cleanup policy validation tests (matching Go SDK's oneof=always never eventually)

#[test]
fn test_validate_container_cleanup_valid() {
    for cleanup in &["always", "never", "eventually"] {
        let mut result = ValidationResult::new();
        validate_container_cleanup(cleanup, "container", &mut result);
        assert!(result.is_valid(), "'{}' should be valid cleanup", cleanup);
    }
}

#[test]
fn test_validate_container_cleanup_invalid() {
    let mut result = ValidationResult::new();
    validate_container_cleanup("sometimes", "container", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].message.contains("sometimes"));
}

// Script language validation tests (matching Go SDK's oneof=javascript js python)

#[test]
fn test_validate_script_language_valid() {
    for lang in &["javascript", "js", "python"] {
        let mut result = ValidationResult::new();
        validate_script_language(lang, "script", &mut result);
        assert!(result.is_valid(), "'{}' should be valid language", lang);
    }
}

#[test]
fn test_validate_script_language_invalid() {
    let mut result = ValidationResult::new();
    validate_script_language("ruby", "script", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].message.contains("ruby"));
}

// Run task validation integration tests

#[test]
fn test_validate_run_task_container_invalid_pull_policy() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "test-run", "1.0.0", None, None, None,
    ));
    workflow.do_.add(
        "runContainer".to_string(),
        TaskDefinition::Run(Box::new(RunTaskDefinition {
            run: ProcessTypeDefinition {
                container: Some(ContainerProcessDefinition {
                    image: "nginx:latest".to_string(),
                    pull_policy: Some("invalid-policy".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            common: TaskDefinitionFields::new(),
        })),
    );
    let result = validate_workflow(&workflow);
    assert!(
        !result.is_valid(),
        "invalid pullPolicy should fail validation"
    );
    assert!(result.errors.iter().any(|e| e.field.contains("pullPolicy")));
}

#[test]
fn test_validate_run_task_container_invalid_cleanup() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "test-run", "1.0.0", None, None, None,
    ));
    workflow.do_.add(
        "runContainer".to_string(),
        TaskDefinition::Run(Box::new(RunTaskDefinition {
            run: ProcessTypeDefinition {
                container: Some(ContainerProcessDefinition {
                    image: "nginx:latest".to_string(),
                    lifetime: Some(ContainerLifetimeDefinition {
                        cleanup: "sometimes".to_string(),
                        after: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            common: TaskDefinitionFields::new(),
        })),
    );
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid(), "invalid cleanup should fail validation");
    assert!(result.errors.iter().any(|e| e.field.contains("cleanup")));
}

#[test]
fn test_validate_run_task_script_invalid_language() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "test-run", "1.0.0", None, None, None,
    ));
    workflow.do_.add(
        "runScript".to_string(),
        TaskDefinition::Run(Box::new(RunTaskDefinition {
            run: ProcessTypeDefinition {
                script: Some(ScriptProcessDefinition {
                    language: "ruby".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            common: TaskDefinitionFields::new(),
        })),
    );
    let result = validate_workflow(&workflow);
    assert!(
        !result.is_valid(),
        "invalid language should fail validation"
    );
    assert!(result.errors.iter().any(|e| e.field.contains("language")));
}

#[test]
fn test_validate_run_task_valid_container() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "test-run", "1.0.0", None, None, None,
    ));
    workflow.do_.add(
        "runContainer".to_string(),
        TaskDefinition::Run(Box::new(RunTaskDefinition {
            run: ProcessTypeDefinition {
                container: Some(ContainerProcessDefinition {
                    image: "nginx:latest".to_string(),
                    pull_policy: Some("always".to_string()),
                    lifetime: Some(ContainerLifetimeDefinition {
                        cleanup: "eventually".to_string(),
                        after: Some(OneOfDurationOrIso8601Expression::Iso8601Expression(
                            "PT5M".to_string(),
                        )),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            common: TaskDefinitionFields::new(),
        })),
    );
    let result = validate_workflow(&workflow);
    assert!(
        result.is_valid(),
        "valid container should pass, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_oauth2_client_auth_method_valid() {
    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("client_secret_basic", "test", &mut result);
    assert!(result.is_valid());

    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("client_secret_post", "test", &mut result);
    assert!(result.is_valid());

    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("client_secret_jwt", "test", &mut result);
    assert!(result.is_valid());

    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("private_key_jwt", "test", &mut result);
    assert!(result.is_valid());

    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("none", "test", &mut result);
    assert!(result.is_valid());

    // Empty string is allowed (optional field)
    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("", "test", &mut result);
    assert!(result.is_valid());
}

#[test]
fn test_validate_oauth2_client_auth_method_invalid() {
    let mut result = ValidationResult::new();
    validate_oauth2_client_auth_method("invalid_method", "test", &mut result);
    assert!(!result.is_valid());
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("client.authentication")));
}

#[test]
fn test_validate_oauth2_request_encoding_valid() {
    let mut result = ValidationResult::new();
    validate_oauth2_request_encoding("application/x-www-form-urlencoded", "test", &mut result);
    assert!(result.is_valid());

    let mut result = ValidationResult::new();
    validate_oauth2_request_encoding("application/json", "test", &mut result);
    assert!(result.is_valid());

    // Empty string is allowed (optional field)
    let mut result = ValidationResult::new();
    validate_oauth2_request_encoding("", "test", &mut result);
    assert!(result.is_valid());
}

#[test]
fn test_validate_oauth2_request_encoding_invalid() {
    let mut result = ValidationResult::new();
    validate_oauth2_request_encoding("text/plain", "test", &mut result);
    assert!(!result.is_valid());
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("request.encoding")));
}

// OIDC authentication mutual exclusion tests (symmetric to OAuth2)

#[test]
fn test_validate_oidc_auth_use_only() {
    let oidc = OpenIDConnectSchemeDefinition {
        use_: Some("oidcSecret".to_string()),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oidc_auth(&oidc, "auth", &mut result);
    assert!(result.is_valid(), "use-only oidc should be valid");
}

#[test]
fn test_validate_oidc_auth_properties_only() {
    let oidc = OpenIDConnectSchemeDefinition {
        use_: None,
        authority: Some("https://auth.example.com/token".to_string()),
        grant: Some("client_credentials".to_string()),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oidc_auth(&oidc, "auth", &mut result);
    assert!(result.is_valid(), "properties-only oidc should be valid");
}

#[test]
fn test_validate_oidc_auth_mutual_exclusion() {
    let oidc = OpenIDConnectSchemeDefinition {
        use_: Some("oidcSecret".to_string()),
        authority: Some("https://auth.example.com/token".to_string()),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oidc_auth(&oidc, "auth", &mut result);
    assert!(!result.is_valid(), "use + properties should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_oidc_auth_neither_set() {
    let oidc = OpenIDConnectSchemeDefinition {
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_oidc_auth(&oidc, "auth", &mut result);
    assert!(!result.is_valid(), "empty oidc should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::Required));
}

#[test]
fn test_validate_auth_policy_with_oidc() {
    let policy = AuthenticationPolicyDefinition {
        use_: None,
        basic: None,
        bearer: None,
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: Some(OpenIDConnectSchemeDefinition {
            use_: None,
            authority: Some("https://auth.example.com/token".to_string()),
            grant: Some("client_credentials".to_string()),
            ..Default::default()
        }),
    };
    let mut result = ValidationResult::new();
    validate_auth_policy(&policy, "auth", &mut result);
    assert!(result.is_valid(), "valid oidc auth policy should pass");
}

// OAuth2 grant type validation tests (matching Go SDK's oneof validation)

#[test]
fn test_validate_oauth2_grant_type_valid() {
    for grant in &[
        "authorization_code",
        "client_credentials",
        "password",
        "refresh_token",
        "urn:ietf:params:oauth:grant-type:token-exchange",
    ] {
        let mut result = ValidationResult::new();
        validate_oauth2_grant_type(grant, "auth.oauth2", &mut result);
        assert!(result.is_valid(), "'{}' should be valid grant type", grant);
    }
    // Empty string is allowed (optional field)
    let mut result = ValidationResult::new();
    validate_oauth2_grant_type("", "auth.oauth2", &mut result);
    assert!(result.is_valid());
}

#[test]
fn test_validate_oauth2_grant_type_invalid() {
    let mut result = ValidationResult::new();
    validate_oauth2_grant_type("invalid_grant", "auth.oauth2", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|e| e.field.contains("grant")));
}

// Authentication policy oneOf validation tests (matching Go SDK's AuthenticationPolicy.UnmarshalJSON)

#[test]
fn test_validate_auth_policy_one_of_single_scheme() {
    let policy = AuthenticationPolicyDefinition {
        use_: None,
        basic: Some(BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
        }),
        bearer: None,
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };
    let mut result = ValidationResult::new();
    validate_auth_policy_one_of(&policy, "auth", &mut result);
    assert!(result.is_valid(), "single scheme should be valid");
}

#[test]
fn test_validate_auth_policy_one_of_multiple_schemes() {
    let policy = AuthenticationPolicyDefinition {
        use_: None,
        basic: Some(BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
        }),
        bearer: Some(BearerAuthenticationSchemeDefinition {
            use_: None,
            token: Some("mytoken".to_string()),
        }),
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };
    let mut result = ValidationResult::new();
    validate_auth_policy_one_of(&policy, "auth", &mut result);
    assert!(!result.is_valid(), "multiple schemes should be invalid");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_auth_policy_one_of_no_schemes() {
    let policy = AuthenticationPolicyDefinition {
        use_: None,
        basic: None,
        bearer: None,
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };
    let mut result = ValidationResult::new();
    validate_auth_policy_one_of(&policy, "auth", &mut result);
    assert!(
        result.is_valid(),
        "no schemes is valid (use_ reference could be set separately)"
    );
}

// Runtime expression helper validation tests

#[test]
fn test_is_non_empty_string() {
    // Plain strings
    assert!(is_non_empty_string("hello"));
    assert!(is_non_empty_string("https://example.com"));
    // Runtime expressions
    assert!(is_non_empty_string("${.foo}"));
    assert!(is_non_empty_string("${ .bar }"));
    // Invalid
    assert!(!is_non_empty_string(""));
}

#[test]
fn test_is_uri_or_runtime_expr() {
    // Literal URIs (no placeholders)
    assert!(is_uri_or_runtime_expr("https://example.com/api"));
    assert!(is_uri_or_runtime_expr("http://localhost:8080"));
    assert!(is_uri_or_runtime_expr("grpc://api.example.com:443"));
    // URI templates (with placeholders)
    assert!(is_uri_or_runtime_expr("http://example.com/{id}"));
    assert!(is_uri_or_runtime_expr(
        "https://api.example.com/v1/users/{userId}/orders/{orderId}"
    ));
    // Runtime expressions
    assert!(is_uri_or_runtime_expr("${.endpoint}"));
    // Invalid
    assert!(!is_uri_or_runtime_expr(""));
    assert!(!is_uri_or_runtime_expr("not-a-uri"));
    assert!(!is_uri_or_runtime_expr("example.com")); // missing scheme
}

#[test]
fn test_is_json_pointer_or_runtime_expr() {
    // JSON Pointers (RFC 6901)
    // Note: empty string "" references whole document per RFC 6901, but function returns false for empty
    // (matching Go SDK behavior where empty = omitempty = not validated)
    assert!(!is_json_pointer_or_runtime_expr(""));
    assert!(is_json_pointer_or_runtime_expr("/"));
    assert!(is_json_pointer_or_runtime_expr("/foo/bar"));
    assert!(is_json_pointer_or_runtime_expr("/items/0/name"));
    assert!(is_json_pointer_or_runtime_expr("/a~1b")); // escaped / in token
    assert!(is_json_pointer_or_runtime_expr("/a~0b")); // escaped ~ in token
                                                       // Runtime expressions
    assert!(is_json_pointer_or_runtime_expr("${.pointer}"));
    // Invalid (doesn't start with /)
    assert!(!is_json_pointer_or_runtime_expr("foo"));
    // Invalid escape sequences
    assert!(!is_json_pointer_or_runtime_expr("/a~2b")); // ~2 is not a valid escape
}

// URI/URI-template pattern tests (matching Go SDK's LiteralUriPattern/LiteralUriTemplatePattern)

#[test]
fn test_is_valid_uri() {
    // Valid literal URIs
    assert!(is_valid_uri("https://example.com/api"));
    assert!(is_valid_uri("http://localhost:8080"));
    assert!(is_valid_uri("grpc://api.example.com:443"));
    assert!(is_valid_uri("ftp://files.example.com/document.pdf"));
    // URI templates are NOT valid literal URIs
    assert!(!is_valid_uri("http://example.com/{id}"));
    // Invalid
    assert!(!is_valid_uri(""));
    assert!(!is_valid_uri("example.com"));
    assert!(!is_valid_uri("not a uri"));
    assert!(!is_valid_uri("://missing-scheme.com"));
}

#[test]
fn test_is_valid_uri_template() {
    // Valid URI templates
    assert!(is_valid_uri_template("http://example.com/{id}"));
    assert!(is_valid_uri_template(
        "https://api.example.com/v1/{resource}/{id}"
    ));
    // Literal URIs are NOT valid URI templates (no placeholders)
    assert!(!is_valid_uri_template("https://example.com/api"));
    // Invalid
    assert!(!is_valid_uri_template(""));
    assert!(!is_valid_uri_template("example.com/{id}"));
}

#[test]
fn test_is_valid_json_pointer() {
    // RFC 6901 JSON Pointers
    assert!(is_valid_json_pointer("")); // whole document
    assert!(is_valid_json_pointer("/"));
    assert!(is_valid_json_pointer("/foo"));
    assert!(is_valid_json_pointer("/foo/bar"));
    assert!(is_valid_json_pointer("/items/0/name"));
    assert!(is_valid_json_pointer("/a~0b")); // escaped ~
    assert!(is_valid_json_pointer("/a~1b")); // escaped /
                                             // Invalid
    assert!(!is_valid_json_pointer("foo")); // no leading /
    assert!(!is_valid_json_pointer("/a~2b")); // invalid escape
    assert!(!is_valid_json_pointer("/a~")); // incomplete escape
}

// Schedule oneOf validation tests (matching Go SDK's mutual exclusivity)

#[test]
fn test_validate_schedule_one_of_single_every() {
    let schedule = WorkflowScheduleDefinition {
        every: Some(Duration {
            hours: Some(1),
            ..Default::default()
        }),
        cron: None,
        after: None,
        on: None,
    };
    let mut result = ValidationResult::new();
    validate_schedule_one_of(&schedule, "schedule", &mut result);
    assert!(result.is_valid(), "single 'every' should be valid");
}

#[test]
fn test_validate_schedule_one_of_single_cron() {
    let schedule = WorkflowScheduleDefinition {
        every: None,
        cron: Some("0 0 * * *".to_string()),
        after: None,
        on: None,
    };
    let mut result = ValidationResult::new();
    validate_schedule_one_of(&schedule, "schedule", &mut result);
    assert!(result.is_valid(), "single 'cron' should be valid");
}

#[test]
fn test_validate_schedule_one_of_multiple() {
    let schedule = WorkflowScheduleDefinition {
        every: Some(Duration {
            hours: Some(1),
            ..Default::default()
        }),
        cron: Some("0 0 * * *".to_string()),
        after: None,
        on: None,
    };
    let mut result = ValidationResult::new();
    validate_schedule_one_of(&schedule, "schedule", &mut result);
    assert!(
        !result.is_valid(),
        "both 'every' and 'cron' should be invalid"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_schedule_one_of_none() {
    let schedule = WorkflowScheduleDefinition {
        every: None,
        cron: None,
        after: None,
        on: None,
    };
    let mut result = ValidationResult::new();
    validate_schedule_one_of(&schedule, "schedule", &mut result);
    assert!(
        result.is_valid(),
        "empty schedule is valid (not required field)"
    );
}

// ProcessType oneOf validation tests (matching Go SDK's RunTaskConfiguration.UnmarshalJSON)

#[test]
fn test_validate_process_type_one_of_shell() {
    let process = ProcessTypeDefinition {
        shell: Some(ShellProcessDefinition {
            command: "echo hello".to_string(),
            ..Default::default()
        }),
        container: None,
        script: None,
        workflow: None,
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_process_type_one_of(&process, "task1", &mut result);
    assert!(result.is_valid(), "single 'shell' should be valid");
}

#[test]
fn test_validate_process_type_one_of_multiple() {
    let process = ProcessTypeDefinition {
        shell: Some(ShellProcessDefinition {
            command: "echo hello".to_string(),
            ..Default::default()
        }),
        container: Some(ContainerProcessDefinition {
            image: "nginx:latest".to_string(),
            ..Default::default()
        }),
        script: None,
        workflow: None,
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_process_type_one_of(&process, "task1", &mut result);
    assert!(
        !result.is_valid(),
        "both 'shell' and 'container' should be invalid"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_process_type_one_of_none() {
    let process = ProcessTypeDefinition {
        shell: None,
        container: None,
        script: None,
        workflow: None,
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_process_type_one_of(&process, "task1", &mut result);
    assert!(
        !result.is_valid(),
        "no process type should be invalid (exactly one required)"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

// Backoff strategy oneOf validation tests (matching Go SDK's RetryBackoff.UnmarshalJSON)

#[test]
fn test_validate_backoff_one_of_single_constant() {
    let backoff = BackoffStrategyDefinition {
        constant: Some(ConstantBackoffDefinition::with_delay("PT5S")),
        exponential: None,
        linear: None,
    };
    let mut result = ValidationResult::new();
    validate_backoff_one_of(&backoff, "backoff", &mut result);
    assert!(result.is_valid(), "single 'constant' should be valid");
}

#[test]
fn test_validate_backoff_one_of_single_exponential() {
    let backoff = BackoffStrategyDefinition {
        constant: None,
        exponential: Some(ExponentialBackoffDefinition::with_factor(2.0)),
        linear: None,
    };
    let mut result = ValidationResult::new();
    validate_backoff_one_of(&backoff, "backoff", &mut result);
    assert!(result.is_valid(), "single 'exponential' should be valid");
}

#[test]
fn test_validate_backoff_one_of_multiple() {
    let backoff = BackoffStrategyDefinition {
        constant: Some(ConstantBackoffDefinition::with_delay("PT5S")),
        exponential: Some(ExponentialBackoffDefinition::with_factor(2.0)),
        linear: None,
    };
    let mut result = ValidationResult::new();
    validate_backoff_one_of(&backoff, "backoff", &mut result);
    assert!(
        !result.is_valid(),
        "both 'constant' and 'exponential' should be invalid"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_backoff_one_of_none() {
    let backoff = BackoffStrategyDefinition {
        constant: None,
        exponential: None,
        linear: None,
    };
    let mut result = ValidationResult::new();
    validate_backoff_one_of(&backoff, "backoff", &mut result);
    assert!(result.is_valid(), "empty backoff is valid (optional field)");
}

// Schema oneOf validation tests (matching Go SDK's Schema.UnmarshalJSON)

#[test]
fn test_validate_schema_one_of_document_only() {
    let schema = SchemaDefinition {
        document: Some(serde_json::json!({"type": "object"})),
        resource: None,
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_schema_one_of(&schema, "schema", &mut result);
    assert!(result.is_valid(), "document-only schema should be valid");
}

#[test]
fn test_validate_schema_one_of_resource_only() {
    let schema = SchemaDefinition {
        document: None,
        resource: Some(ExternalResourceDefinition {
            name: Some("schema".to_string()),
            endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                "https://example.com/schema.json".to_string(),
            ),
        }),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_schema_one_of(&schema, "schema", &mut result);
    assert!(result.is_valid(), "resource-only schema should be valid");
}

#[test]
fn test_validate_schema_one_of_both_set() {
    let schema = SchemaDefinition {
        document: Some(serde_json::json!({"type": "object"})),
        resource: Some(ExternalResourceDefinition {
            name: Some("schema".to_string()),
            endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                "https://example.com/schema.json".to_string(),
            ),
        }),
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_schema_one_of(&schema, "schema", &mut result);
    assert!(
        !result.is_valid(),
        "both document and resource should be invalid"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::MutualExclusion));
}

#[test]
fn test_validate_schema_one_of_neither_set() {
    let schema = SchemaDefinition {
        document: None,
        resource: None,
        ..Default::default()
    };
    let mut result = ValidationResult::new();
    validate_schema_one_of(&schema, "schema", &mut result);
    assert!(
        !result.is_valid(),
        "neither document nor resource should be invalid"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::Required));
}

// Extension task type validation tests (matching Go SDK's oneof=call composite emit for listen raise run set switch try wait all)

#[test]
fn test_validate_extension_task_type_valid() {
    for task_type in &[
        "call",
        "composite",
        "emit",
        "for",
        "listen",
        "raise",
        "run",
        "set",
        "switch",
        "try",
        "wait",
        "all",
    ] {
        let mut result = ValidationResult::new();
        validate_extension_task_type(task_type, "extension", &mut result);
        assert!(
            result.is_valid(),
            "'{}' should be valid extension task type",
            task_type
        );
    }
}

#[test]
fn test_validate_extension_task_type_invalid() {
    let mut result = ValidationResult::new();
    validate_extension_task_type("invalid_type", "extension", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].message.contains("invalid_type"));
}

// HTTP method validation tests (matching Go SDK's oneofci=GET POST PUT DELETE PATCH)

#[test]
fn test_validate_http_method_valid() {
    for method in &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] {
        let mut result = ValidationResult::new();
        validate_http_method(method, "task1", &mut result);
        assert!(
            result.is_valid(),
            "'{}' should be valid HTTP method",
            method
        );
    }
}

#[test]
fn test_validate_http_method_case_insensitive() {
    // Go SDK's oneofci validation is case-insensitive
    let mut result = ValidationResult::new();
    validate_http_method("get", "task1", &mut result);
    assert!(
        result.is_valid(),
        "'get' (lowercase) should be valid HTTP method"
    );

    let mut result = ValidationResult::new();
    validate_http_method("Post", "task1", &mut result);
    assert!(
        result.is_valid(),
        "'Post' (mixed case) should be valid HTTP method"
    );
}

#[test]
fn test_validate_http_method_invalid() {
    let mut result = ValidationResult::new();
    validate_http_method("CONNECT", "task1", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].message.contains("CONNECT"));
}

// Container lifetime validation tests (matching Go SDK's required_if=Cleanup eventually)

#[test]
fn test_validate_container_lifetime_eventually_with_after() {
    let lifetime = ContainerLifetimeDefinition {
        cleanup: "eventually".to_string(),
        after: Some(OneOfDurationOrIso8601Expression::Iso8601Expression(
            "PT5M".to_string(),
        )),
    };
    let mut result = ValidationResult::new();
    validate_container_lifetime(&lifetime, "container", &mut result);
    assert!(result.is_valid(), "eventually with after should be valid");
}

#[test]
fn test_validate_container_lifetime_eventually_without_after() {
    let lifetime = ContainerLifetimeDefinition {
        cleanup: "eventually".to_string(),
        after: None,
    };
    let mut result = ValidationResult::new();
    validate_container_lifetime(&lifetime, "container", &mut result);
    assert!(
        !result.is_valid(),
        "eventually without after should be invalid"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("lifetime.after")));
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::Required));
}

#[test]
fn test_validate_container_lifetime_always_no_after_needed() {
    let lifetime = ContainerLifetimeDefinition {
        cleanup: "always".to_string(),
        after: None,
    };
    let mut result = ValidationResult::new();
    validate_container_lifetime(&lifetime, "container", &mut result);
    assert!(result.is_valid(), "always cleanup doesn't require after");
}

#[test]
fn test_validate_container_lifetime_invalid_cleanup() {
    let lifetime = ContainerLifetimeDefinition {
        cleanup: "sometimes".to_string(),
        after: None,
    };
    let mut result = ValidationResult::new();
    validate_container_lifetime(&lifetime, "container", &mut result);
    assert!(!result.is_valid(), "invalid cleanup should fail");
    assert!(result.errors.iter().any(|e| e.field.contains("cleanup")));
}

// Document name/version format validation tests (matching Go SDK's validate tags)

#[test]
fn test_validate_document_name_hostname() {
    // Go SDK: validate:"required,hostname_rfc1123"
    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata {
        dsl: "1.0.0".to_string(),
        namespace: "default".to_string(),
        name: "invalid name!".to_string(), // spaces and ! not valid in hostname
        version: "1.0.0".to_string(),
        title: None,
        summary: None,
        tags: None,
        metadata: None,
    });
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid(), "invalid hostname name should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field == "document.name" && e.rule == ValidationRule::Hostname));
}

#[test]
fn test_validate_document_version_semver() {
    // Go SDK: validate:"required,semver_pattern"
    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata {
        dsl: "1.0.0".to_string(),
        namespace: "default".to_string(),
        name: "test-workflow".to_string(),
        version: "not-semver".to_string(),
        title: None,
        summary: None,
        tags: None,
        metadata: None,
    });
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid(), "invalid semver version should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field == "document.version" && e.rule == ValidationRule::Semver));
}

#[test]
fn test_validate_document_name_valid_hostname() {
    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "my-workflow",
        "1.0.0",
        None,
        None,
        None,
    ));
    let result = validate_workflow(&workflow);
    assert!(
        result.errors.iter().all(|e| e.field != "document.name"),
        "valid hostname name should pass"
    );
}

#[test]
fn test_validate_document_version_valid_semver() {
    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "test-workflow",
        "2.3.4",
        None,
        None,
        None,
    ));
    let result = validate_workflow(&workflow);
    assert!(
        result.errors.iter().all(|e| e.field != "document.version"),
        "valid semver version should pass"
    );
}

// HTTP output format validation tests (matching Go SDK's oneof=raw content response)

#[test]
fn test_validate_http_output_valid() {
    for output in &["raw", "content", "response"] {
        let mut result = ValidationResult::new();
        validate_http_output(output, "task1.with", &mut result);
        assert!(
            result.is_valid(),
            "'{}' should be valid HTTP output format",
            output
        );
    }
}

#[test]
fn test_validate_http_output_empty_valid() {
    // Empty string is valid (omitempty - optional field)
    let mut result = ValidationResult::new();
    validate_http_output("", "task1.with", &mut result);
    assert!(result.is_valid(), "empty output should be valid (optional)");
}

#[test]
fn test_validate_http_output_invalid() {
    let mut result = ValidationResult::new();
    validate_http_output("invalid", "task1.with", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].message.contains("invalid"));
}

// AsyncAPI protocol validation tests (matching Go SDK's oneof validation)

#[test]
fn test_validate_asyncapi_protocol_valid() {
    for protocol in &[
        "amqp", "amqp1", "http", "kafka", "mqtt", "mqtt5", "nats", "redis", "ws",
    ] {
        let mut result = ValidationResult::new();
        validate_asyncapi_protocol(protocol, "task1.with", &mut result);
        assert!(
            result.is_valid(),
            "'{}' should be valid AsyncAPI protocol",
            protocol
        );
    }
}

#[test]
fn test_validate_asyncapi_protocol_empty_valid() {
    let mut result = ValidationResult::new();
    validate_asyncapi_protocol("", "task1.with", &mut result);
    assert!(
        result.is_valid(),
        "empty protocol should be valid (optional)"
    );
}

#[test]
fn test_validate_asyncapi_protocol_invalid() {
    let mut result = ValidationResult::new();
    validate_asyncapi_protocol("ftp", "task1.with", &mut result);
    assert!(!result.is_valid());
    assert!(result.errors[0].field.contains("protocol"));
}

// GRPC host hostname validation test (matching Go SDK's hostname_rfc1123)

#[test]
fn test_validate_grpc_host_hostname() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "test-grpc",
        "1.0.0",
        None,
        None,
        None,
    ));
    workflow.do_.add(
        "callGRPC".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::GRPC(Box::new(
            crate::models::call::CallGRPCDefinition {
                call: "grpc".to_string(),
                with: crate::models::call::GRPCArguments {
                    proto: ExternalResourceDefinition {
                        name: Some("proto".to_string()),
                        endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                            "https://example.com/proto.proto".to_string(),
                        ),
                    },
                    service: crate::models::call::GRPCServiceDefinition {
                        name: "myservice".to_string(),
                        host: "invalid host!".to_string(),
                        port: None,
                        authentication: None,
                    },
                    method: "MyMethod".to_string(),
                    arguments: None,
                    authentication: None,
                },
                common: TaskDefinitionFields::new(),
            },
        )))),
    );
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid(), "invalid GRPC host should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("service.host") && e.rule == ValidationRule::Hostname));
}

// Switch task validation tests (matching Go SDK's switch_item + then required validation)

#[test]
fn test_validate_switch_task_valid() {
    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some(".status == 200".to_string()),
            then: Some("nextTask".to_string()),
        },
    );
    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        common: TaskDefinitionFields::new(),
    };
    let mut result = ValidationResult::new();
    validate_switch_task(&switch_task, "task1", &mut result);
    assert!(
        result.is_valid(),
        "valid switch task should pass, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_switch_task_empty_cases() {
    let switch_task = SwitchTaskDefinition {
        switch: Map::new(),
        common: TaskDefinitionFields::new(),
    };
    let mut result = ValidationResult::new();
    validate_switch_task(&switch_task, "task1", &mut result);
    assert!(!result.is_valid(), "empty switch cases should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("switch") && e.rule == ValidationRule::Required));
}

#[test]
fn test_validate_switch_task_case_without_then() {
    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some(".status == 200".to_string()),
            then: None, // Missing required 'then'
        },
    );
    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        common: TaskDefinitionFields::new(),
    };
    let mut result = ValidationResult::new();
    validate_switch_task(&switch_task, "task1", &mut result);
    assert!(!result.is_valid(), "switch case without 'then' should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("then") && e.rule == ValidationRule::Required));
}

#[test]
fn test_validate_switch_task_default_case_with_then() {
    // Default case (no when) with then is valid
    let mut switch_cases = Map::new();
    switch_cases.add(
        "default".to_string(),
        SwitchCaseDefinition {
            when: None,
            then: Some("end".to_string()),
        },
    );
    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        common: TaskDefinitionFields::new(),
    };
    let mut result = ValidationResult::new();
    validate_switch_task(&switch_task, "task1", &mut result);
    assert!(result.is_valid(), "default case with then should be valid");
}

#[test]
fn test_validate_switch_task_multiple_cases() {
    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some(".x == 1".to_string()),
            then: Some("task2".to_string()),
        },
    );
    switch_cases.add(
        "case2".to_string(),
        SwitchCaseDefinition {
            when: Some(".x == 2".to_string()),
            then: Some("task3".to_string()),
        },
    );
    switch_cases.add(
        "default".to_string(),
        SwitchCaseDefinition {
            when: None,
            then: Some("end".to_string()),
        },
    );
    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        common: TaskDefinitionFields::new(),
    };
    let mut result = ValidationResult::new();
    validate_switch_task(&switch_task, "task1", &mut result);
    assert!(
        result.is_valid(),
        "multiple cases should be valid, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_switch_task_integration_empty_in_workflow() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "test-switch",
        "1.0.0",
        None,
        None,
        None,
    ));
    workflow.do_.add(
        "emptySwitch".to_string(),
        TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map::new(),
            common: TaskDefinitionFields::new(),
        }),
    );
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid(), "empty switch should fail validation");
    assert!(result.errors.iter().any(|e| e.field.contains("switch")));
}

// WorkflowProcessDefinition validation tests (matching Go SDK's required,hostname_rfc1123,semver_pattern)

#[test]
fn test_validate_workflow_process_valid() {
    let workflow_process = crate::models::task::WorkflowProcessDefinition {
        namespace: "default".to_string(),
        name: "child-workflow".to_string(),
        version: "1.0.0".to_string(),
        input: None,
    };
    let mut result = ValidationResult::new();
    validate_workflow_process(&workflow_process, "task1", &mut result);
    assert!(
        result.is_valid(),
        "valid workflow process should pass, got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_workflow_process_invalid_namespace() {
    let workflow_process = crate::models::task::WorkflowProcessDefinition {
        namespace: "invalid namespace!".to_string(),
        name: "child-workflow".to_string(),
        version: "1.0.0".to_string(),
        input: None,
    };
    let mut result = ValidationResult::new();
    validate_workflow_process(&workflow_process, "task1", &mut result);
    assert!(!result.is_valid(), "invalid namespace should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("namespace") && e.rule == ValidationRule::Hostname));
}

#[test]
fn test_validate_workflow_process_invalid_name() {
    let workflow_process = crate::models::task::WorkflowProcessDefinition {
        namespace: "default".to_string(),
        name: "invalid name!".to_string(),
        version: "1.0.0".to_string(),
        input: None,
    };
    let mut result = ValidationResult::new();
    validate_workflow_process(&workflow_process, "task1", &mut result);
    assert!(!result.is_valid(), "invalid name should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("name") && e.rule == ValidationRule::Hostname));
}

#[test]
fn test_validate_workflow_process_invalid_version() {
    let workflow_process = crate::models::task::WorkflowProcessDefinition {
        namespace: "default".to_string(),
        name: "child-workflow".to_string(),
        version: "not-semver".to_string(),
        input: None,
    };
    let mut result = ValidationResult::new();
    validate_workflow_process(&workflow_process, "task1", &mut result);
    assert!(!result.is_valid(), "invalid version should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("version") && e.rule == ValidationRule::Semver));
}

#[test]
fn test_validate_workflow_process_empty_fields() {
    let workflow_process = crate::models::task::WorkflowProcessDefinition {
        namespace: "".to_string(),
        name: "".to_string(),
        version: "".to_string(),
        input: None,
    };
    let mut result = ValidationResult::new();
    validate_workflow_process(&workflow_process, "task1", &mut result);
    assert!(!result.is_valid(), "empty fields should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::Required));
}

// GRPC host required validation test (matching Go SDK's validate:"required,hostname_rfc1123")

#[test]
fn test_validate_grpc_host_required() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "test-grpc",
        "1.0.0",
        None,
        None,
        None,
    ));
    workflow.do_.add(
        "callGRPC".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::GRPC(Box::new(
            crate::models::call::CallGRPCDefinition {
                call: "grpc".to_string(),
                with: crate::models::call::GRPCArguments {
                    proto: ExternalResourceDefinition {
                        name: Some("proto".to_string()),
                        endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                            "https://example.com/proto.proto".to_string(),
                        ),
                    },
                    service: crate::models::call::GRPCServiceDefinition {
                        name: "myservice".to_string(),
                        host: "".to_string(), // Empty host should fail
                        port: None,
                        authentication: None,
                    },
                    method: "MyMethod".to_string(),
                    arguments: None,
                    authentication: None,
                },
                common: TaskDefinitionFields::new(),
            },
        )))),
    );
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid(), "empty GRPC host should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.field.contains("service.host") && e.rule == ValidationRule::Required));
}

// Set task validation tests (matching Go SDK's validate:"required,min=1,dive")

#[test]
fn test_validate_set_task_map_with_values() {
    use crate::models::task::SetValue;
    let mut map = HashMap::new();
    map.insert("key".to_string(), serde_json::json!("value"));
    let set = SetValue::Map(map);
    let mut result = ValidationResult::new();
    validate_set_task(&set, "task1", &mut result);
    assert!(result.is_valid(), "set with values should pass");
}

#[test]
fn test_validate_set_task_empty_map() {
    use crate::models::task::SetValue;
    let set = SetValue::Map(HashMap::new());
    let mut result = ValidationResult::new();
    validate_set_task(&set, "task1", &mut result);
    assert!(!result.is_valid(), "empty set map should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.rule == ValidationRule::Required));
}

#[test]
fn test_validate_set_task_expression() {
    use crate::models::task::SetValue;
    let set = SetValue::Expression("${ .data }".to_string());
    let mut result = ValidationResult::new();
    validate_set_task(&set, "task1", &mut result);
    assert!(result.is_valid(), "set with expression should pass");
}

#[test]
fn test_validate_set_task_empty_expression() {
    use crate::models::task::SetValue;
    let set = SetValue::Expression("".to_string());
    let mut result = ValidationResult::new();
    validate_set_task(&set, "task1", &mut result);
    assert!(!result.is_valid(), "empty set expression should fail");
}
