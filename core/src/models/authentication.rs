use serde::{Deserialize, Serialize};

/// Provides the default OAuth2 request encoding
fn default_oauth2_request_encoding() -> String {
    OAuth2RequestEncoding::FORM_URL.to_string()
}

/// Provides the default OAUTH2 token endpoint
fn default_token_endpoint() -> String {
    "/oauth2/token".to_string()
}

/// Provides the default OAUTH2 revocation endpoint
fn default_revocation_endpoint() -> String {
    "/oauth2/revoke".to_string()
}

/// Provides the default OAUTH2 introspection endpoint
fn default_introspection_endpoint() -> String {
    "/oauth2/introspect".to_string()
}

string_constants! {
    /// Enumerates all supported authentication schemes
    AuthenticationScheme {
        BASIC => "Basic",
        BEARER => "Bearer",
        CERTIFICATE => "Certificate",
        DIGEST => "Digest",
        OAUTH2 => "OAuth2",
        OIDC => "OpenIDConnect",
    }
}

string_constants! {
    /// Enumerates all supported OAUTH2 authentication methods
    OAuth2ClientAuthenticationMethod {
        BASIC => "client_secret_basic",
        POST => "client_secret_post",
        JWT => "client_secret_jwt",
        PRIVATE_KEY => "private_key_jwt",
        NONE => "none",
    }
}

string_constants! {
    /// Exposes all supported request encodings for OAUTH2 requests
    OAuth2RequestEncoding {
        FORM_URL => "application/x-www-form-urlencoded",
        JSON => "application/json",
    }
}

/// Represents the definition of an authentication policy
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticationPolicyDefinition {
    /// Gets/sets the name of the top level authentication policy to use, if any
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,

    /// Gets/sets the `basic` authentication scheme to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic: Option<BasicAuthenticationSchemeDefinition>,

    /// Gets/sets the `Bearer` authentication scheme to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer: Option<BearerAuthenticationSchemeDefinition>,

    /// Gets/sets the `Certificate` authentication scheme to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<CertificateAuthenticationSchemeDefinition>,

    /// Gets/sets the `Digest` authentication scheme to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<DigestAuthenticationSchemeDefinition>,

    /// Gets/sets the `OAUTH2` authentication scheme to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<OAuth2AuthenticationSchemeDefinition>,

    /// Gets/sets the `OIDC` authentication scheme to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc: Option<OpenIDConnectSchemeDefinition>,
}

/// Represents a referenceable authentication policy that can either be a reference or an inline policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReferenceableAuthenticationPolicy {
    /// A reference to a named authentication policy
    Reference(AuthenticationPolicyReference),
    /// An inline authentication policy definition
    Policy(Box<AuthenticationPolicyDefinition>),
}

/// Represents a reference to a named authentication policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticationPolicyReference {
    /// The name of the authentication policy to use
    #[serde(rename = "use")]
    pub use_: String,
}

/// Macro to define a username/password credential authentication scheme.
/// Used by Basic and Digest schemes which share the same field structure.
macro_rules! credential_auth_scheme {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name {
            /// Gets/sets the name of the secret, if any, used to configure the authentication scheme
            #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
            pub use_: Option<String>,

            /// Gets/sets the username used for authentication
            #[serde(skip_serializing_if = "Option::is_none")]
            pub username: Option<String>,

            /// Gets/sets the password used for authentication
            #[serde(skip_serializing_if = "Option::is_none")]
            pub password: Option<String>,
        }
    };
}

credential_auth_scheme!(
    /// Represents the definition of a basic authentication scheme
    BasicAuthenticationSchemeDefinition
);

/// Represents the definition of a bearer authentication scheme
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BearerAuthenticationSchemeDefinition {
    /// Gets/sets the name of the secret, if any, used to configure the authentication scheme
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,

    /// Gets/sets the bearer token used for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Represents the definition of a certificate authentication scheme
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateAuthenticationSchemeDefinition {
    /// Gets/sets the name of the secret, if any, used to configure the authentication scheme
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
}

credential_auth_scheme!(
    /// Represents the definition of a digest authentication scheme
    DigestAuthenticationSchemeDefinition
);

/// Represents the definition of an OAUTH2 client
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2AuthenticationClientDefinition {
    /// Gets/sets the OAUTH2 `client_id` to use. Required if 'Authentication' has NOT been set to 'none'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Gets/sets the OAUTH2 `client_secret` to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,

    /// Gets/sets a JWT, if any, containing a signed assertion with the application credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertion: Option<String>,

    /// Gets/sets the authentication method to use to authenticate the client. Defaults to 'client_secret_post'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<String>,
}

/// Represents the configuration of an OAUTH2 authentication request
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2AuthenticationRequestDefinition {
    /// Gets/sets the encoding of the authentication request. Defaults to 'application/x-www-form-urlencoded'
    #[serde(default = "default_oauth2_request_encoding")]
    pub encoding: String,
}

/// Represents the definition of an OAUTH2 token
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2TokenDefinition {
    /// Gets/sets the security token to use
    pub token: String,

    /// Gets/sets the type of security token to use
    #[serde(rename = "type")]
    pub type_: String,
}

/// Represents the configuration of OAUTH2 endpoints
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2AuthenticationEndpointsDefinition {
    /// Gets/sets the relative path to the token endpoint. Defaults to `/oauth2/token`
    #[serde(default = "default_token_endpoint")]
    pub token: String,

    /// Gets/sets the relative path to the revocation endpoint. Defaults to `/oauth2/revoke`
    #[serde(default = "default_revocation_endpoint")]
    pub revocation: String,

    /// Gets/sets the relative path to the introspection endpoint. Defaults to `/oauth2/introspect`
    #[serde(default = "default_introspection_endpoint")]
    pub introspection: String,
}

/// Macro to define OAuth2-like authentication scheme structs.
/// OAuth2 and OIDC share the same field set except OAuth2 has an extra `endpoints` field.
macro_rules! oauth2_like_auth_scheme {
    ($( #[$meta:meta] )* $name:ident { $($extra_field:tt)* }) => {
        $( #[$meta] )*
        #[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name {
            /// Gets/sets the name of the secret, if any, used to configure the authentication scheme
            #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
            pub use_: Option<String>,

            $($extra_field)*

            /// Gets/sets the URI that references the OAUTH2 authority to use.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub authority: Option<String>,

            /// Gets/sets the grant type to use.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub grant: Option<String>,

            /// Gets/sets the definition of the client to use.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub client: Option<OAuth2AuthenticationClientDefinition>,

            /// Gets/sets the configuration of the authentication request to perform.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub request: Option<OAuth2AuthenticationRequestDefinition>,

            /// Gets/sets a list of valid issuers for token checks.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub issuers: Option<Vec<String>>,

            /// Gets/sets the scopes to request the token for.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub scopes: Option<Vec<String>>,

            /// Gets/sets the audiences to request the token for.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub audiences: Option<Vec<String>>,

            /// Gets/sets the username to use (for Password grant).
            #[serde(skip_serializing_if = "Option::is_none")]
            pub username: Option<String>,

            /// Gets/sets the password to use (for Password grant).
            #[serde(skip_serializing_if = "Option::is_none")]
            pub password: Option<String>,

            /// Gets/sets the token representing the identity of the party on whose behalf the request is made.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub subject: Option<OAuth2TokenDefinition>,

            /// Gets/sets the token representing the acting party's identity.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub actor: Option<OAuth2TokenDefinition>,
        }
    };
}

oauth2_like_auth_scheme!(
    /// Represents the definition of an OAUTH2 authentication scheme
    OAuth2AuthenticationSchemeDefinition {
        /// Gets/sets the configuration of the OAUTH2 endpoints to use
        #[serde(skip_serializing_if = "Option::is_none")]
        pub endpoints: Option<OAuth2AuthenticationEndpointsDefinition>,
    }
);

oauth2_like_auth_scheme!(
    /// Represents the definition of an OpenIDConnect authentication scheme
    OpenIDConnectSchemeDefinition {}
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth_serialize() {
        let basic = BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("john".to_string()),
            password: Some("12345".to_string()),
        };
        let json = serde_json::to_string(&basic).unwrap();
        assert!(json.contains("\"username\":\"john\""));
        assert!(json.contains("\"password\":\"12345\""));
        assert!(!json.contains("\"use\""));
    }

    #[test]
    fn test_basic_auth_deserialize() {
        let json = r#"{"username": "admin", "password": "secret"}"#;
        let basic: BasicAuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(basic.username, Some("admin".to_string()));
        assert_eq!(basic.password, Some("secret".to_string()));
    }

    #[test]
    fn test_basic_auth_with_use_secret() {
        let json = r#"{"use": "mySecret"}"#;
        let basic: BasicAuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(basic.use_, Some("mySecret".to_string()));
        assert!(basic.username.is_none());
    }

    #[test]
    fn test_bearer_auth_serialize() {
        let bearer = BearerAuthenticationSchemeDefinition {
            use_: None,
            token: Some("mytoken123".to_string()),
        };
        let json = serde_json::to_string(&bearer).unwrap();
        assert!(json.contains("\"token\":\"mytoken123\""));
    }

    #[test]
    fn test_bearer_auth_with_use_secret() {
        let json = r#"{"use": "bearerSecret"}"#;
        let bearer: BearerAuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(bearer.use_, Some("bearerSecret".to_string()));
    }

    #[test]
    fn test_digest_auth_serialize() {
        let digest = DigestAuthenticationSchemeDefinition {
            use_: None,
            username: Some("digestUser".to_string()),
            password: Some("digestPass".to_string()),
        };
        let json = serde_json::to_string(&digest).unwrap();
        assert!(json.contains("\"username\":\"digestUser\""));
        assert!(json.contains("\"password\":\"digestPass\""));
    }

    #[test]
    fn test_auth_policy_basic() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
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
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("\"basic\""));
        assert!(json.contains("\"username\":\"john\""));
        assert!(!json.contains("\"bearer\""));
    }

    #[test]
    fn test_auth_policy_digest_deserialize() {
        let json = r#"{
            "digest": {
                "username": "digestUser",
                "password": "digestPass"
            }
        }"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.digest.is_some());
        assert!(policy.basic.is_none());
        let digest = policy.digest.unwrap();
        assert_eq!(digest.username, Some("digestUser".to_string()));
        assert_eq!(digest.password, Some("digestPass".to_string()));
    }

    #[test]
    fn test_auth_policy_oauth2_inline() {
        let json = r#"{
            "oauth2": {
                "authority": "https://auth.example.com",
                "grant": "client_credentials",
                "scopes": ["scope1", "scope2"]
            }
        }"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.oauth2.is_some());
        let oauth2 = policy.oauth2.unwrap();
        assert_eq!(
            oauth2.authority,
            Some("https://auth.example.com".to_string())
        );
        assert_eq!(oauth2.grant, Some("client_credentials".to_string()));
        assert_eq!(
            oauth2.scopes,
            Some(vec!["scope1".to_string(), "scope2".to_string()])
        );
    }

    #[test]
    fn test_auth_policy_oauth2_use() {
        let json = r#"{
            "oauth2": {
                "use": "mysecret"
            }
        }"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.oauth2.is_some());
        let oauth2 = policy.oauth2.unwrap();
        assert_eq!(oauth2.use_, Some("mysecret".to_string()));
    }

    #[test]
    fn test_referenceable_auth_policy_reference() {
        let json = r#"{"use": "myAuthPolicy"}"#;
        let ref_policy: ReferenceableAuthenticationPolicy = serde_json::from_str(json).unwrap();
        match ref_policy {
            ReferenceableAuthenticationPolicy::Reference(r) => {
                assert_eq!(r.use_, "myAuthPolicy");
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn test_referenceable_auth_policy_inline() {
        let json = r#"{
            "basic": {
                "username": "john",
                "password": "secret"
            }
        }"#;
        let ref_policy: ReferenceableAuthenticationPolicy = serde_json::from_str(json).unwrap();
        match ref_policy {
            ReferenceableAuthenticationPolicy::Policy(p) => {
                assert!(p.basic.is_some());
            }
            _ => panic!("Expected Policy variant"),
        }
    }

    #[test]
    fn test_oauth2_scheme_roundtrip() {
        let json = r#"{
            "authority": "https://auth.example.com",
            "grant": "client_credentials",
            "scopes": ["scope1", "scope2"]
        }"#;
        let oauth2: OAuth2AuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&oauth2).unwrap();
        let deserialized: OAuth2AuthenticationSchemeDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(oauth2, deserialized);
    }

    // Additional tests matching Go SDK's authentication_test.go

    #[test]
    fn test_auth_policy_basic_roundtrip() {
        // Matches Go SDK's TestAuthenticationPolicy "Valid Basic Authentication Inline"
        let json = r#"{"basic":{"username":"john","password":"12345"}}"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.basic.is_some());
        let basic = policy.basic.as_ref().unwrap();
        assert_eq!(basic.username, Some("john".to_string()));
        assert_eq!(basic.password, Some("12345".to_string()));
        let serialized = serde_json::to_string(&policy).unwrap();
        let deserialized: AuthenticationPolicyDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_auth_policy_digest_roundtrip() {
        // Matches Go SDK's TestAuthenticationPolicy "Valid Digest Authentication Inline"
        let json = r#"{"digest":{"username":"digestUser","password":"digestPass"}}"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.digest.is_some());
        let digest = policy.digest.as_ref().unwrap();
        assert_eq!(digest.username, Some("digestUser".to_string()));
        assert_eq!(digest.password, Some("digestPass".to_string()));
        let serialized = serde_json::to_string(&policy).unwrap();
        let deserialized: AuthenticationPolicyDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_oauth2_use_secret() {
        // Matches Go SDK's TestAuthenticationOAuth2Policy "Valid OAuth2 Authentication Use"
        let json = r#"{"oauth2":{"use":"mysecret"}}"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.oauth2.is_some());
        let oauth2 = policy.oauth2.as_ref().unwrap();
        assert_eq!(oauth2.use_, Some("mysecret".to_string()));
    }

    #[test]
    fn test_oauth2_inline_properties() {
        // Matches Go SDK's TestAuthenticationOAuth2Policy "Valid OAuth2 Authentication Inline"
        let json = r#"{"oauth2":{"authority":"https://auth.example.com","grant":"client_credentials","scopes":["scope1","scope2"]}}"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.oauth2.is_some());
        let oauth2 = policy.oauth2.as_ref().unwrap();
        assert_eq!(
            oauth2.authority,
            Some("https://auth.example.com".to_string())
        );
        assert_eq!(oauth2.grant, Some("client_credentials".to_string()));
        assert_eq!(
            oauth2.scopes,
            Some(vec!["scope1".to_string(), "scope2".to_string()])
        );
    }

    #[test]
    fn test_bearer_auth_roundtrip() {
        let bearer = BearerAuthenticationSchemeDefinition {
            use_: None,
            token: Some("mytoken123".to_string()),
        };
        let serialized = serde_json::to_string(&bearer).unwrap();
        let deserialized: BearerAuthenticationSchemeDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(bearer, deserialized);
    }

    #[test]
    fn test_referenceable_auth_policy_roundtrip() {
        // Test inline auth policy roundtrip through ReferenceableAuthenticationPolicy
        let json = r#"{"basic":{"username":"admin","password":"admin"}}"#;
        let ref_policy: ReferenceableAuthenticationPolicy = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&ref_policy).unwrap();
        let deserialized: ReferenceableAuthenticationPolicy =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(ref_policy, deserialized);
    }

    #[test]
    fn test_basic_auth_use_vs_credentials() {
        // Basic auth can have either "use" (secret reference) or username/password
        let use_json = r#"{"use":"mySecret"}"#;
        let basic_use: BasicAuthenticationSchemeDefinition =
            serde_json::from_str(use_json).unwrap();
        assert_eq!(basic_use.use_, Some("mySecret".to_string()));
        assert!(basic_use.username.is_none());

        let cred_json = r#"{"username":"admin","password":"secret"}"#;
        let basic_cred: BasicAuthenticationSchemeDefinition =
            serde_json::from_str(cred_json).unwrap();
        assert!(basic_cred.use_.is_none());
        assert_eq!(basic_cred.username, Some("admin".to_string()));
        assert_eq!(basic_cred.password, Some("secret".to_string()));
    }

    #[test]
    fn test_digest_auth_use_secret() {
        let json = r#"{"use":"digestSecret"}"#;
        let digest: DigestAuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(digest.use_, Some("digestSecret".to_string()));
        assert!(digest.username.is_none());
    }

    // Additional tests matching Go SDK's authentication_oauth_test.go and authentication_test.go

    #[test]
    fn test_oauth2_full_properties() {
        // Matches Go SDK's TestOAuth2AuthenticationPolicyValidation - valid properties
        let json = r#"{
            "authority": "https://auth.example.com",
            "grant": "client_credentials",
            "scopes": ["scope1", "scope2"],
            "client": {
                "id": "my-client-id",
                "secret": "my-client-secret",
                "authentication": "client_secret_post"
            },
            "request": {
                "encoding": "application/x-www-form-urlencoded"
            },
            "issuers": ["https://issuer1.example.com", "https://issuer2.example.com"],
            "audiences": ["api1", "api2"],
            "endpoints": {
                "token": "/oauth2/token",
                "revocation": "/oauth2/revoke",
                "introspection": "/oauth2/introspect"
            }
        }"#;
        let oauth2: OAuth2AuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(
            oauth2.authority,
            Some("https://auth.example.com".to_string())
        );
        assert_eq!(oauth2.grant, Some("client_credentials".to_string()));
        assert_eq!(
            oauth2.scopes,
            Some(vec!["scope1".to_string(), "scope2".to_string()])
        );
        assert!(oauth2.client.is_some());
        let client = oauth2.client.as_ref().unwrap();
        assert_eq!(client.id, Some("my-client-id".to_string()));
        assert_eq!(client.secret, Some("my-client-secret".to_string()));
        assert_eq!(
            client.authentication,
            Some("client_secret_post".to_string())
        );
        assert!(oauth2.request.is_some());
        let req = oauth2.request.as_ref().unwrap();
        assert_eq!(req.encoding, "application/x-www-form-urlencoded");
        assert_eq!(
            oauth2.issuers,
            Some(vec![
                "https://issuer1.example.com".to_string(),
                "https://issuer2.example.com".to_string()
            ])
        );
        assert_eq!(
            oauth2.audiences,
            Some(vec!["api1".to_string(), "api2".to_string()])
        );
        assert!(oauth2.endpoints.is_some());
        let endpoints = oauth2.endpoints.as_ref().unwrap();
        assert_eq!(endpoints.token, "/oauth2/token");
        assert_eq!(endpoints.revocation, "/oauth2/revoke");
        assert_eq!(endpoints.introspection, "/oauth2/introspect");
    }

    #[test]
    fn test_oauth2_full_properties_roundtrip() {
        let json = r#"{
            "authority": "https://auth.example.com",
            "grant": "client_credentials",
            "scopes": ["scope1"],
            "client": {"id": "client1", "secret": "secret1"}
        }"#;
        let oauth2: OAuth2AuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&oauth2).unwrap();
        let deserialized: OAuth2AuthenticationSchemeDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(oauth2, deserialized);
    }

    #[test]
    fn test_oauth2_password_grant() {
        // OAuth2 with password grant type
        let json = r#"{
            "authority": "https://auth.example.com",
            "grant": "password",
            "username": "user1",
            "password": "pass1",
            "scopes": ["read", "write"]
        }"#;
        let oauth2: OAuth2AuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(oauth2.grant, Some("password".to_string()));
        assert_eq!(oauth2.username, Some("user1".to_string()));
        assert_eq!(oauth2.password, Some("pass1".to_string()));
    }

    #[test]
    fn test_oauth2_token_exchange_grant() {
        // OAuth2 with token exchange (urn:ietf:params:oauth:grant-type:token-exchange)
        let json = r#"{
            "authority": "https://auth.example.com",
            "grant": "urn:ietf:params:oauth:grant-type:token-exchange",
            "subject": {"token": "subject-token", "type": "urn:ietf:params:oauth:token-type:access_token"},
            "actor": {"token": "actor-token", "type": "urn:ietf:params:oauth:token-type:access_token"}
        }"#;
        let oauth2: OAuth2AuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(
            oauth2.grant,
            Some("urn:ietf:params:oauth:grant-type:token-exchange".to_string())
        );
        assert!(oauth2.subject.is_some());
        let subject = oauth2.subject.as_ref().unwrap();
        assert_eq!(subject.token, "subject-token");
        assert_eq!(
            subject.type_,
            "urn:ietf:params:oauth:token-type:access_token"
        );
        assert!(oauth2.actor.is_some());
        let actor = oauth2.actor.as_ref().unwrap();
        assert_eq!(actor.token, "actor-token");
    }

    #[test]
    fn test_oauth2_endpoints_defaults() {
        // OAuth2 endpoints serde defaults (applied during deserialization, not Default trait)
        let json = r#"{}"#;
        let endpoints: OAuth2AuthenticationEndpointsDefinition =
            serde_json::from_str(json).unwrap();
        assert_eq!(endpoints.token, "/oauth2/token");
        assert_eq!(endpoints.revocation, "/oauth2/revoke");
        assert_eq!(endpoints.introspection, "/oauth2/introspect");
    }

    #[test]
    fn test_oauth2_request_encoding_defaults() {
        // OAuth2 request encoding serde defaults (applied during deserialization, not Default trait)
        let json = r#"{}"#;
        let request: OAuth2AuthenticationRequestDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(request.encoding, "application/x-www-form-urlencoded");
    }

    #[test]
    fn test_oauth2_request_encoding_json() {
        let json = r#"{"encoding": "application/json"}"#;
        let request: OAuth2AuthenticationRequestDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(request.encoding, "application/json");
    }

    #[test]
    fn test_oidc_full_properties() {
        // OIDC shares similar structure with OAuth2
        let json = r#"{
            "authority": "https://oidc.example.com",
            "grant": "authorization_code",
            "scopes": ["openid", "profile"],
            "client": {"id": "oidc-client"}
        }"#;
        let oidc: OpenIDConnectSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(oidc.authority, Some("https://oidc.example.com".to_string()));
        assert_eq!(oidc.grant, Some("authorization_code".to_string()));
        assert_eq!(
            oidc.scopes,
            Some(vec!["openid".to_string(), "profile".to_string()])
        );
        assert!(oidc.client.is_some());
    }

    #[test]
    fn test_oidc_roundtrip() {
        let json = r#"{
            "authority": "https://oidc.example.com",
            "grant": "authorization_code",
            "scopes": ["openid"]
        }"#;
        let oidc: OpenIDConnectSchemeDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&oidc).unwrap();
        let deserialized: OpenIDConnectSchemeDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(oidc, deserialized);
    }

    #[test]
    fn test_certificate_auth() {
        let json = r#"{"use": "certSecret"}"#;
        let cert: CertificateAuthenticationSchemeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(cert.use_, Some("certSecret".to_string()));
        assert_eq!(AuthenticationScheme::CERTIFICATE, "Certificate");
    }

    #[test]
    fn test_auth_policy_bearer_inline() {
        // Full auth policy with bearer token
        let json = r#"{
            "bearer": {
                "token": "my-bearer-token"
            }
        }"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.bearer.is_some());
        let bearer = policy.bearer.as_ref().unwrap();
        assert_eq!(bearer.token, Some("my-bearer-token".to_string()));
        assert!(bearer.use_.is_none());
    }

    #[test]
    fn test_auth_policy_certificate_inline() {
        let json = r#"{
            "certificate": {
                "use": "myCertSecret"
            }
        }"#;
        let policy: AuthenticationPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.certificate.is_some());
        let cert = policy.certificate.as_ref().unwrap();
        assert_eq!(cert.use_, Some("myCertSecret".to_string()));
    }

    #[test]
    fn test_oauth2_client_assertion() {
        // OAuth2 client with JWT assertion
        let json = r#"{
            "id": "client-id",
            "assertion": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
            "authentication": "private_key_jwt"
        }"#;
        let client: OAuth2AuthenticationClientDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(client.id, Some("client-id".to_string()));
        assert_eq!(
            client.assertion,
            Some("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...".to_string())
        );
        assert_eq!(client.authentication, Some("private_key_jwt".to_string()));
    }

    #[test]
    fn test_oauth2_token_definition() {
        let json = r#"{
            "token": "my-token-value",
            "type": "urn:ietf:params:oauth:token-type:access_token"
        }"#;
        let token: OAuth2TokenDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(token.token, "my-token-value");
        assert_eq!(token.type_, "urn:ietf:params:oauth:token-type:access_token");
    }
}
