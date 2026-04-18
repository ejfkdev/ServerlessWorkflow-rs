use crate::models::authentication::*;
use serde::{Deserialize, Serialize};

/// Represents the definition of an external resource
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalResourceDefinition {
    /// Gets/sets the external resource's name, if any
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Gets/sets the endpoint at which to get the defined resource
    #[serde(rename = "endpoint")]
    pub endpoint: OneOfEndpointDefinitionOrUri,
}

/// Represents the definition of an endpoint
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointDefinition {
    /// Gets/sets the endpoint's uri
    #[serde(rename = "uri")]
    pub uri: String,

    /// Gets/sets the endpoint's authentication policy, if any
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ReferenceableAuthenticationPolicy>,
}

/// Represents a value that can be either an EndpointDefinition or an Uri
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOfEndpointDefinitionOrUri {
    /// Variant holding an EndpointDefinition
    Endpoint(Box<EndpointDefinition>),
    /// Variant holding a URL
    Uri(String),
}
impl Default for OneOfEndpointDefinitionOrUri {
    fn default() -> Self {
        // Choose a default variant. For example, default to an empty Uri.
        OneOfEndpointDefinitionOrUri::Uri(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_uri_serialize() {
        let endpoint = OneOfEndpointDefinitionOrUri::Uri("http://example.com/{id}".to_string());
        let json = serde_json::to_string(&endpoint).unwrap();
        assert_eq!(json, r#""http://example.com/{id}""#);
    }

    #[test]
    fn test_endpoint_uri_deserialize() {
        let json = r#""http://example.com/api""#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        match endpoint {
            OneOfEndpointDefinitionOrUri::Uri(uri) => {
                assert_eq!(uri, "http://example.com/api");
            }
            _ => panic!("Expected Uri variant"),
        }
    }

    #[test]
    fn test_endpoint_config_deserialize() {
        let json = r#"{
            "uri": "http://example.com/{id}",
            "authentication": {
                "basic": { "username": "admin", "password": "admin" }
            }
        }"#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        match endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(ep) => {
                assert_eq!(ep.uri, "http://example.com/{id}");
                assert!(ep.authentication.is_some());
            }
            _ => panic!("Expected Endpoint variant"),
        }
    }

    #[test]
    fn test_endpoint_config_with_oauth2_reference() {
        let json = r#"{
            "uri": "http://example.com/{id}",
            "authentication": {
                "oauth2": { "use": "secret" }
            }
        }"#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        match endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(ep) => {
                assert_eq!(ep.uri, "http://example.com/{id}");
                assert!(ep.authentication.is_some());
            }
            _ => panic!("Expected Endpoint variant"),
        }
    }

    #[test]
    fn test_endpoint_config_roundtrip() {
        let json = r#"{
            "uri": "http://example.com/{id}",
            "authentication": {
                "basic": { "username": "admin", "password": "admin" }
            }
        }"#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&endpoint).unwrap();
        let deserialized: OneOfEndpointDefinitionOrUri = serde_json::from_str(&serialized).unwrap();
        assert_eq!(endpoint, deserialized);
    }

    #[test]
    fn test_external_resource_deserialize() {
        let json = r#"{
            "name": "myResource",
            "endpoint": "https://api.example.com/data"
        }"#;
        let resource: ExternalResourceDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(resource.name, Some("myResource".to_string()));
        match resource.endpoint {
            OneOfEndpointDefinitionOrUri::Uri(uri) => {
                assert_eq!(uri, "https://api.example.com/data");
            }
            _ => panic!("Expected Uri variant"),
        }
    }

    #[test]
    fn test_runtime_expression_endpoint() {
        let json = r#""${example}""#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        match endpoint {
            OneOfEndpointDefinitionOrUri::Uri(expr) => {
                assert_eq!(expr, "${example}");
            }
            _ => panic!("Expected Uri variant for runtime expression"),
        }
    }

    // Additional tests matching Go SDK's endpoint_test.go patterns

    #[test]
    fn test_endpoint_uri_template() {
        // Matches Go SDK's TestEndpoint_UnmarshalJSON "Valid URITemplate"
        let json = r#""http://example.com/{id}""#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        match endpoint {
            OneOfEndpointDefinitionOrUri::Uri(uri) => {
                assert_eq!(uri, "http://example.com/{id}");
            }
            _ => panic!("Expected Uri variant"),
        }
    }

    #[test]
    fn test_endpoint_config_with_basic_auth_roundtrip() {
        // Matches Go SDK's TestEndpoint_MarshalJSON "Marshal EndpointConfiguration"
        let json = r#"{
            "uri": "http://example.com/{id}",
            "authentication": {
                "basic": {"username": "john", "password": "secret"}
            }
        }"#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&endpoint).unwrap();
        let deserialized: OneOfEndpointDefinitionOrUri = serde_json::from_str(&serialized).unwrap();
        assert_eq!(endpoint, deserialized);
    }

    #[test]
    fn test_endpoint_config_with_oauth2_use_roundtrip() {
        // Matches Go SDK's TestEndpoint_UnmarshalJSON "Valid EndpointConfiguration with reference"
        let json = r#"{
            "uri": "http://example.com/{id}",
            "authentication": {
                "oauth2": {"use": "secret"}
            }
        }"#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&endpoint).unwrap();
        let deserialized: OneOfEndpointDefinitionOrUri = serde_json::from_str(&serialized).unwrap();
        assert_eq!(endpoint, deserialized);
    }

    #[test]
    fn test_endpoint_config_with_runtime_expression_uri() {
        // Matches Go SDK's TestEndpoint_UnmarshalJSON "Valid EndpointConfiguration with reference and expression"
        let json = r#"{
            "uri": "${example}",
            "authentication": {
                "oauth2": {"use": "secret"}
            }
        }"#;
        let endpoint: OneOfEndpointDefinitionOrUri = serde_json::from_str(json).unwrap();
        match endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(ep) => {
                assert_eq!(ep.uri, "${example}");
                assert!(ep.authentication.is_some());
            }
            _ => panic!("Expected Endpoint variant"),
        }
    }

    #[test]
    fn test_endpoint_runtime_expression_roundtrip() {
        // Matches Go SDK's TestEndpoint_MarshalJSON "Marshal RuntimeExpression"
        let endpoint = OneOfEndpointDefinitionOrUri::Uri("${example}".to_string());
        let serialized = serde_json::to_string(&endpoint).unwrap();
        assert_eq!(serialized, r#""${example}""#);
        let deserialized: OneOfEndpointDefinitionOrUri = serde_json::from_str(&serialized).unwrap();
        assert_eq!(endpoint, deserialized);
    }

    #[test]
    fn test_external_resource_with_endpoint_config() {
        // External resource with endpoint configuration (not just URI string)
        let json = r#"{
            "name": "myResource",
            "endpoint": {
                "uri": "http://example.com/api",
                "authentication": {
                    "basic": {"username": "admin", "password": "admin"}
                }
            }
        }"#;
        let resource: ExternalResourceDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(resource.name, Some("myResource".to_string()));
        match resource.endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(ep) => {
                assert_eq!(ep.uri, "http://example.com/api");
                assert!(ep.authentication.is_some());
            }
            _ => panic!("Expected Endpoint variant"),
        }
    }

    #[test]
    fn test_endpoint_default() {
        let default = OneOfEndpointDefinitionOrUri::default();
        match default {
            OneOfEndpointDefinitionOrUri::Uri(s) => assert!(s.is_empty()),
            _ => panic!("Expected default Uri variant"),
        }
    }
}
