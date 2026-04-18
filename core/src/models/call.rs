use super::task::TaskDefinitionFields;
use crate::models::authentication::*;
use crate::models::resource::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

string_constants! {
    /// Enumerates all supported call types
    CallType {
        ASYNCAPI => "asyncapi",
        GRPC => "grpc",
        HTTP => "http",
        OPENAPI => "openapi",
        A2A => "a2a",
    }
}

string_constants! {
    /// Enumerates all supported AsyncAPI protocols
    AsyncApiProtocol {
        AMQP => "amqp",
        AMQP1 => "amqp1",
        ANYPOINTMQ => "anypointmq",
        GOOGLE_PUBSUB => "googlepubsub",
        HTTP => "http",
        IBMMQ => "ibmmq",
        JMS => "jms",
        KAFKA => "kafka",
        MERCURE => "mercure",
        MQTT => "mqtt",
        MQTT5 => "mqtt5",
        NATS => "nats",
        PULSAR => "pulsar",
        REDIS => "redis",
        SNS => "sns",
        SOLACE => "solace",
        SQS => "sqs",
        STOMP => "stomp",
        WS => "ws",
    }
}

string_constants! {
    /// Enumerates all supported A2A methods
    A2AMethod {
        MESSAGE_SEND => "message/send",
        MESSAGE_STREAM => "message/stream",
        TASKS_GET => "tasks/get",
        TASKS_LIST => "tasks/list",
        TASKS_CANCEL => "tasks/cancel",
        TASKS_RESUBSCRIBE => "tasks/resubscribe",
        TASKS_PUSH_NOTIFICATION_CONFIG_SET => "tasks/pushNotificationConfig/set",
        TASKS_PUSH_NOTIFICATION_CONFIG_GET => "tasks/pushNotificationConfig/get",
        TASKS_PUSH_NOTIFICATION_CONFIG_LIST => "tasks/pushNotificationConfig/list",
        TASKS_PUSH_NOTIFICATION_CONFIG_DELETE => "tasks/pushNotificationConfig/delete",
        AGENT_GET_AUTHENTICATED_EXTENDED_CARD => "agent/getAuthenticatedExtendedCard",
    }
}

/// Represents a value that can be any of the supported call task definitions
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CallTaskDefinition {
    /// Variant holding the definition of an AsyncAPI call
    AsyncAPI(CallAsyncAPIDefinition),
    /// Variant holding the definition of a GRPC call
    GRPC(Box<CallGRPCDefinition>),
    /// Variant holding the definition of an HTTP call
    HTTP(CallHTTPDefinition),
    /// Variant holding the definition of an OpenAPI call
    OpenAPI(CallOpenAPIDefinition),
    /// Variant holding the definition of an A2A call
    A2A(CallA2ADefinition),
    /// Variant holding the definition of a function call
    Function(CallFunctionDefinition),
}

impl CallTaskDefinition {
    /// Returns the common fields shared by all call task definition variants.
    pub fn common_fields(&self) -> &super::task::TaskDefinitionFields {
        match self {
            CallTaskDefinition::HTTP(t) => &t.common,
            CallTaskDefinition::GRPC(t) => &t.common,
            CallTaskDefinition::OpenAPI(t) => &t.common,
            CallTaskDefinition::AsyncAPI(t) => &t.common,
            CallTaskDefinition::A2A(t) => &t.common,
            CallTaskDefinition::Function(t) => &t.common,
        }
    }
}

impl<'de> serde::Deserialize<'de> for CallTaskDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        let call_value = value.get("call").and_then(|v| v.as_str()).unwrap_or("");

        macro_rules! try_call {
            ($variant:ident, $ty:ty) => {
                <$ty>::deserialize(value)
                    .map(CallTaskDefinition::$variant)
                    .map_err(serde::de::Error::custom)
            };
        }
        macro_rules! try_call_boxed {
            ($variant:ident, $ty:ty) => {
                <$ty>::deserialize(value)
                    .map(|v| CallTaskDefinition::$variant(Box::new(v)))
                    .map_err(serde::de::Error::custom)
            };
        }

        match call_value {
            "asyncapi" => try_call!(AsyncAPI, CallAsyncAPIDefinition),
            "grpc" => try_call_boxed!(GRPC, CallGRPCDefinition),
            "http" => try_call!(HTTP, CallHTTPDefinition),
            "openapi" => try_call!(OpenAPI, CallOpenAPIDefinition),
            "a2a" => try_call!(A2A, CallA2ADefinition),
            _ => try_call!(Function, CallFunctionDefinition),
        }
    }
}

/// Represents the HTTP call arguments
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct HTTPArguments {
    /// Gets/sets the HTTP method of the HTTP request to perform
    #[serde(rename = "method")]
    pub method: String,

    /// Gets/sets the HTTP endpoint to send the request to
    #[serde(rename = "endpoint")]
    pub endpoint: super::resource::OneOfEndpointDefinitionOrUri,

    /// Gets/sets a name/value mapping of the headers, if any, of the HTTP request to perform
    #[serde(rename = "headers", skip_serializing_if = "Option::is_none")]
    pub headers: Option<OneOfHeadersOrExpression>,

    /// Gets/sets the body, if any, of the HTTP request to perform
    #[serde(rename = "body", skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,

    /// Gets/sets a name/value mapping of the query parameters, if any, of the HTTP request to perform
    #[serde(rename = "query", skip_serializing_if = "Option::is_none")]
    pub query: Option<OneOfQueryOrExpression>,

    /// Gets/sets the http call output format. Defaults to 'content'
    #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Gets/sets whether redirection status codes (300-399) should be treated as errors
    #[serde(rename = "redirect", skip_serializing_if = "Option::is_none")]
    pub redirect: Option<bool>,
}

/// A value that can be either a string-keyed map or a runtime expression
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringMapOrExpression {
    /// A name/value mapping
    Map(HashMap<String, String>),
    /// A runtime expression
    Expression(String),
}

impl Default for StringMapOrExpression {
    fn default() -> Self {
        StringMapOrExpression::Map(HashMap::new())
    }
}

/// Represents headers that can be either a map or a runtime expression
pub type OneOfHeadersOrExpression = StringMapOrExpression;

/// Represents query parameters that can be either a map or a runtime expression
pub type OneOfQueryOrExpression = StringMapOrExpression;

/// Macro to define a call task definition struct with the common `call`, `with`, and `common` fields.
macro_rules! define_call_definition {
    ($( #[$meta:meta] )* $name:ident, $with_ty:ty) => {
        $( #[$meta] )*
        #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
        pub struct $name {
            /// Gets/sets the call type identifier
            #[serde(rename = "call")]
            pub call: String,

            /// Gets/sets the call arguments
            #[serde(rename = "with")]
            pub with: $with_ty,

            /// Gets/sets the task's common fields
            #[serde(flatten)]
            pub common: TaskDefinitionFields,
        }
    };
}

define_call_definition!(
    /// Represents the definition of an HTTP call task
    CallHTTPDefinition, HTTPArguments
);

/// Represents the GRPC service definition
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GRPCServiceDefinition {
    /// Gets/sets the name of the GRPC service to call
    #[serde(rename = "name")]
    pub name: String,

    /// Gets/sets the hostname of the GRPC service to call
    #[serde(rename = "host")]
    pub host: String,

    /// Gets/sets the port number of the GRPC service to call
    #[serde(rename = "port", skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// Gets/sets the endpoint's authentication policy, if any
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ReferenceableAuthenticationPolicy>,
}

/// Represents the GRPC call arguments
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct GRPCArguments {
    /// Gets/sets the proto resource that describes the GRPC service to call
    #[serde(rename = "proto")]
    pub proto: ExternalResourceDefinition,

    /// Gets/sets the GRPC service definition
    #[serde(rename = "service")]
    pub service: GRPCServiceDefinition,

    /// Gets/sets the name of the method to call on the defined GRPC service
    #[serde(rename = "method")]
    pub method: String,

    /// Gets/sets the arguments, if any, to call the method with
    #[serde(rename = "arguments", skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,

    /// Gets/sets the authentication policy, if any, to use when calling the GRPC service
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ReferenceableAuthenticationPolicy>,
}

define_call_definition!(
    /// Represents the definition of a GRPC call task
    CallGRPCDefinition, GRPCArguments
);

/// Represents the OpenAPI call arguments
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAPIArguments {
    /// Gets/sets the document that defines the OpenAPI operation to call
    #[serde(rename = "document")]
    pub document: ExternalResourceDefinition,

    /// Gets/sets the id of the OpenAPI operation to call
    #[serde(rename = "operationId")]
    pub operation_id: String,

    /// Gets/sets a name/value mapping of the parameters of the OpenAPI operation to call
    #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, Value>>,

    /// Gets/sets the authentication policy, if any, to use when calling the OpenAPI operation
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ReferenceableAuthenticationPolicy>,

    /// Gets/sets the http call output format. Defaults to 'content'
    #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Gets/sets whether redirection status codes (300-399) should be treated as errors
    #[serde(rename = "redirect", skip_serializing_if = "Option::is_none")]
    pub redirect: Option<bool>,
}

define_call_definition!(
    /// Represents the definition of an OpenAPI call task
    CallOpenAPIDefinition, OpenAPIArguments
);

/// Represents the AsyncAPI server configuration
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncApiServerDefinition {
    /// Gets/sets the target server's name
    #[serde(rename = "name")]
    pub name: String,

    /// Gets/sets the target server's variables, if any
    #[serde(rename = "variables", skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, Value>>,
}

/// Represents an AsyncAPI outbound message
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncApiOutboundMessageDefinition {
    /// Gets/sets the message's payload, if any
    #[serde(rename = "payload", skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,

    /// Gets/sets the message's headers, if any
    #[serde(rename = "headers", skip_serializing_if = "Option::is_none")]
    pub headers: Option<Value>,
}

/// Represents an AsyncAPI inbound message (extends outbound with correlationId)
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncApiInboundMessageDefinition {
    /// Gets/sets the message's payload, if any
    #[serde(rename = "payload", skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,

    /// Gets/sets the message's headers, if any
    #[serde(rename = "headers", skip_serializing_if = "Option::is_none")]
    pub headers: Option<Value>,

    /// Gets/sets the message's correlation id, if any
    #[serde(rename = "correlationId", skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

/// Represents an AsyncAPI message consumption policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AsyncApiMessageConsumptionPolicy {
    /// Consume a specific amount of messages
    Amount {
        /// The amount of (filtered) messages to consume
        amount: u32,
    },
    /// Consume for a specified duration
    For {
        /// The duration to consume messages for
        #[serde(rename = "for")]
        for_: super::duration::OneOfDurationOrIso8601Expression,
    },
    /// Consume while a condition is true
    While {
        /// A runtime expression evaluated after each consumed message
        #[serde(rename = "while")]
        while_: String,
    },
    /// Consume until a condition is true
    Until {
        /// A runtime expression evaluated before each consumed message
        until: String,
    },
}

impl Default for AsyncApiMessageConsumptionPolicy {
    fn default() -> Self {
        AsyncApiMessageConsumptionPolicy::Amount { amount: 1 }
    }
}

/// Represents an AsyncAPI subscription
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsyncApiSubscriptionDefinition {
    /// Gets/sets a runtime expression, if any, used to filter consumed messages
    #[serde(rename = "filter", skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    /// Gets/sets the subscription's message consumption policy
    #[serde(rename = "consume")]
    pub consume: AsyncApiMessageConsumptionPolicy,
}

/// Represents the AsyncAPI call arguments
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncApiArguments {
    /// Gets/sets the document that defines the AsyncAPI operation to call
    #[serde(rename = "document")]
    pub document: ExternalResourceDefinition,

    /// Gets/sets the name of the channel (AsyncAPI v2.6.0)
    #[serde(rename = "channel", skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    /// Gets/sets a reference to the AsyncAPI operation to call
    #[serde(rename = "operation", skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,

    /// Gets/sets the server to call the specified AsyncAPI operation on
    #[serde(rename = "server", skip_serializing_if = "Option::is_none")]
    pub server: Option<AsyncApiServerDefinition>,

    /// Gets/sets the protocol to use to select the target server
    #[serde(rename = "protocol", skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,

    /// Gets/sets the message to publish using the target operation
    #[serde(rename = "message", skip_serializing_if = "Option::is_none")]
    pub message: Option<AsyncApiOutboundMessageDefinition>,

    /// Gets/sets the subscription to messages consumed using the target operation
    #[serde(rename = "subscription", skip_serializing_if = "Option::is_none")]
    pub subscription: Option<AsyncApiSubscriptionDefinition>,

    /// Gets/sets the authentication policy, if any, to use when calling the AsyncAPI operation
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ReferenceableAuthenticationPolicy>,
}

define_call_definition!(
    /// Represents the definition of an AsyncAPI call task
    CallAsyncAPIDefinition, AsyncApiArguments
);

/// Represents A2A call parameters that can be either an object or a runtime expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOfA2AParametersOrExpression {
    /// A parameters object
    Map(HashMap<String, Value>),
    /// A runtime expression
    Expression(String),
}

impl Default for OneOfA2AParametersOrExpression {
    fn default() -> Self {
        OneOfA2AParametersOrExpression::Map(HashMap::new())
    }
}

/// Represents the A2A call arguments
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2AArguments {
    /// Gets/sets the Agent Card that defines the agent to call
    #[serde(rename = "agentCard", skip_serializing_if = "Option::is_none")]
    pub agent_card: Option<ExternalResourceDefinition>,

    /// Gets/sets the server endpoint to send the request to
    #[serde(rename = "server", skip_serializing_if = "Option::is_none")]
    pub server: Option<super::resource::OneOfEndpointDefinitionOrUri>,

    /// Gets/sets the A2A method to send
    #[serde(rename = "method")]
    pub method: String,

    /// Gets/sets the parameters object to send with the A2A method
    #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
    pub parameters: Option<OneOfA2AParametersOrExpression>,
}

define_call_definition!(
    /// Represents the definition of an A2A call task
    CallA2ADefinition, A2AArguments
);

/// Represents the definition of a function call task
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallFunctionDefinition {
    /// Gets/sets the name of the function to call
    #[serde(rename = "call")]
    pub call: String,

    /// Gets/sets a name/value mapping of the parameters, if any, to call the function with
    #[serde(rename = "with", skip_serializing_if = "Option::is_none")]
    pub with: Option<HashMap<String, Value>>,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_http_deserialize() {
        let json = r#"{
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": "http://example.com/api"
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(http.call, "http");
        assert_eq!(http.with.method, "GET");
        match &http.with.endpoint {
            OneOfEndpointDefinitionOrUri::Uri(uri) => assert_eq!(uri, "http://example.com/api"),
            _ => panic!("Expected Uri variant"),
        }
    }

    #[test]
    fn test_call_http_with_headers_and_query() {
        let json = r#"{
            "call": "http",
            "with": {
                "method": "POST",
                "endpoint": "http://example.com/api",
                "headers": {"Authorization": "Bearer token"},
                "query": {"page": "1"},
                "output": "response",
                "redirect": true
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(http.with.method, "POST");
        assert!(http.with.headers.is_some());
        assert!(http.with.query.is_some());
        assert_eq!(http.with.output, Some("response".to_string()));
        assert_eq!(http.with.redirect, Some(true));
    }

    #[test]
    fn test_call_http_roundtrip() {
        let json = r#"{
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": "http://example.com/api"
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&http).unwrap();
        let deserialized: CallHTTPDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(http, deserialized);
    }

    #[test]
    fn test_call_http_with_endpoint_config() {
        let json = r#"{
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": {
                    "uri": "http://example.com/{id}",
                    "authentication": {
                        "basic": {"username": "admin", "password": "secret"}
                    }
                }
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        match &http.with.endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(ep) => {
                assert_eq!(ep.uri, "http://example.com/{id}");
                assert!(ep.authentication.is_some());
            }
            _ => panic!("Expected Endpoint variant"),
        }
    }

    #[test]
    fn test_call_grpc_deserialize() {
        let json = r#"{
            "call": "grpc",
            "with": {
                "proto": {
                    "name": "MyProto",
                    "endpoint": "http://example.com/proto"
                },
                "service": {
                    "name": "UserService",
                    "host": "example.com",
                    "port": 50051
                },
                "method": "GetUser",
                "arguments": {"userId": "12345"}
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(grpc.call, "grpc");
        assert_eq!(grpc.with.proto.name, Some("MyProto".to_string()));
        assert_eq!(grpc.with.service.name, "UserService");
        assert_eq!(grpc.with.service.host, "example.com");
        assert_eq!(grpc.with.service.port, Some(50051));
        assert_eq!(grpc.with.method, "GetUser");
    }

    #[test]
    fn test_call_grpc_roundtrip() {
        let json = r#"{
            "call": "grpc",
            "with": {
                "proto": {
                    "name": "MyProto",
                    "endpoint": "http://example.com/proto"
                },
                "service": {
                    "name": "UserService",
                    "host": "example.com",
                    "port": 50051
                },
                "method": "GetUser"
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&grpc).unwrap();
        let deserialized: CallGRPCDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(grpc, deserialized);
    }

    #[test]
    fn test_call_openapi_deserialize() {
        let json = r#"{
            "call": "openapi",
            "with": {
                "document": {
                    "name": "MyOpenAPIDoc",
                    "endpoint": "http://example.com/openapi.json"
                },
                "operationId": "getUsers",
                "parameters": {"param1": "value1"},
                "authentication": {"use": "my-auth"},
                "output": "content",
                "redirect": true
            }
        }"#;
        let openapi: CallOpenAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(openapi.call, "openapi");
        assert_eq!(openapi.with.operation_id, "getUsers");
        assert!(openapi.with.parameters.is_some());
        assert!(openapi.with.authentication.is_some());
        assert_eq!(openapi.with.output, Some("content".to_string()));
        assert_eq!(openapi.with.redirect, Some(true));
    }

    #[test]
    fn test_call_openapi_roundtrip() {
        let json = r#"{
            "call": "openapi",
            "with": {
                "document": {
                    "name": "MyOpenAPIDoc",
                    "endpoint": "http://example.com/openapi.json"
                },
                "operationId": "getUsers"
            }
        }"#;
        let openapi: CallOpenAPIDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&openapi).unwrap();
        let deserialized: CallOpenAPIDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(openapi, deserialized);
    }

    #[test]
    fn test_call_asyncapi_deserialize() {
        let json = r#"{
            "call": "asyncapi",
            "with": {
                "document": {
                    "name": "MyAsyncAPIDoc",
                    "endpoint": "http://example.com/asyncapi.json"
                },
                "operation": "user.signup",
                "server": {"name": "default-server"},
                "protocol": "http",
                "message": {
                    "payload": {"userId": "12345"}
                },
                "authentication": {"use": "asyncapi-auth"}
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(asyncapi.call, "asyncapi");
        assert_eq!(asyncapi.with.operation, Some("user.signup".to_string()));
        assert!(asyncapi.with.server.is_some());
        assert_eq!(asyncapi.with.protocol, Some("http".to_string()));
        assert!(asyncapi.with.message.is_some());
        assert!(asyncapi.with.authentication.is_some());
    }

    #[test]
    fn test_call_asyncapi_roundtrip() {
        let json = r#"{
            "call": "asyncapi",
            "with": {
                "document": {
                    "name": "MyAsyncAPIDoc",
                    "endpoint": "http://example.com/asyncapi.json"
                },
                "operation": "user.signup",
                "protocol": "http"
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&asyncapi).unwrap();
        let deserialized: CallAsyncAPIDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(asyncapi, deserialized);
    }

    #[test]
    fn test_call_function_deserialize() {
        let json = r#"{
            "call": "myFunction",
            "with": {
                "param1": "value1",
                "param2": 42
            }
        }"#;
        let func: CallFunctionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(func.call, "myFunction");
        assert!(func.with.is_some());
        let with = func.with.unwrap();
        assert_eq!(with.get("param1").unwrap(), "value1");
    }

    #[test]
    fn test_call_function_roundtrip() {
        let json = r#"{
            "call": "myFunction",
            "with": {"param1": "value1"}
        }"#;
        let func: CallFunctionDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&func).unwrap();
        let deserialized: CallFunctionDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(func, deserialized);
    }

    #[test]
    fn test_call_a2a_deserialize() {
        let json = r#"{
            "call": "a2a",
            "with": {
                "method": "message/send",
                "parameters": {"message": "hello"}
            }
        }"#;
        let a2a: CallA2ADefinition = serde_json::from_str(json).unwrap();
        assert_eq!(a2a.call, "a2a");
        assert_eq!(a2a.with.method, "message/send");
        assert!(a2a.with.parameters.is_some());
    }

    #[test]
    fn test_call_task_definition_http() {
        let json =
            r#"{"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}"#;
        let def: CallTaskDefinition = serde_json::from_str(json).unwrap();
        match def {
            CallTaskDefinition::HTTP(http) => assert_eq!(http.call, "http"),
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[test]
    fn test_call_task_definition_grpc() {
        let json = r#"{"call": "grpc", "with": {"proto": {"endpoint": "http://example.com/proto"}, "service": {"name": "Svc", "host": "example.com"}, "method": "Get"}}"#;
        let def: CallTaskDefinition = serde_json::from_str(json).unwrap();
        match def {
            CallTaskDefinition::GRPC(grpc) => assert_eq!(grpc.call, "grpc"),
            _ => panic!("Expected GRPC variant"),
        }
    }

    #[test]
    fn test_call_task_definition_openapi() {
        let json = r#"{"call": "openapi", "with": {"document": {"endpoint": "http://example.com/openapi.json"}, "operationId": "op1"}}"#;
        let def: CallTaskDefinition = serde_json::from_str(json).unwrap();
        match def {
            CallTaskDefinition::OpenAPI(openapi) => assert_eq!(openapi.call, "openapi"),
            _ => panic!("Expected OpenAPI variant"),
        }
    }

    #[test]
    fn test_call_task_definition_function() {
        let json = r#"{"call": "myFunc", "with": {"key": "val"}}"#;
        let def: CallTaskDefinition = serde_json::from_str(json).unwrap();
        match def {
            CallTaskDefinition::Function(func) => assert_eq!(func.call, "myFunc"),
            _ => panic!("Expected Function variant"),
        }
    }

    #[test]
    fn test_headers_map_vs_expression() {
        let map_json = r#"{"Authorization": "Bearer token"}"#;
        let map: OneOfHeadersOrExpression = serde_json::from_str(map_json).unwrap();
        assert!(matches!(map, OneOfHeadersOrExpression::Map(_)));

        let expr_json = r#""${ .headers }""#;
        let expr: OneOfHeadersOrExpression = serde_json::from_str(expr_json).unwrap();
        assert!(matches!(expr, OneOfHeadersOrExpression::Expression(_)));
    }

    #[test]
    fn test_query_map_vs_expression() {
        let map_json = r#"{"page": "1"}"#;
        let map: OneOfQueryOrExpression = serde_json::from_str(map_json).unwrap();
        assert!(matches!(map, OneOfQueryOrExpression::Map(_)));

        let expr_json = r#""${ .queryParams }""#;
        let expr: OneOfQueryOrExpression = serde_json::from_str(expr_json).unwrap();
        assert!(matches!(expr, OneOfQueryOrExpression::Expression(_)));
    }

    #[test]
    fn test_asyncapi_consumption_policy_amount() {
        let json = r#"{"amount": 5}"#;
        let policy: AsyncApiMessageConsumptionPolicy = serde_json::from_str(json).unwrap();
        match policy {
            AsyncApiMessageConsumptionPolicy::Amount { amount } => assert_eq!(amount, 5),
            _ => panic!("Expected Amount variant"),
        }
    }

    #[test]
    fn test_asyncapi_consumption_policy_for() {
        let json = r#"{"for": "PT30S"}"#;
        let policy: AsyncApiMessageConsumptionPolicy = serde_json::from_str(json).unwrap();
        match policy {
            AsyncApiMessageConsumptionPolicy::For { for_ } => {
                // Should parse as ISO8601 expression
                assert!(matches!(
                    for_,
                    crate::models::duration::OneOfDurationOrIso8601Expression::Iso8601Expression(_)
                ));
            }
            _ => panic!("Expected For variant"),
        }
    }

    #[test]
    fn test_asyncapi_consumption_policy_for_duration() {
        let json = r#"{"for": {"seconds": 30}}"#;
        let policy: AsyncApiMessageConsumptionPolicy = serde_json::from_str(json).unwrap();
        match policy {
            AsyncApiMessageConsumptionPolicy::For { for_ } => {
                assert!(matches!(
                    for_,
                    crate::models::duration::OneOfDurationOrIso8601Expression::Duration(_)
                ));
            }
            _ => panic!("Expected For variant"),
        }
    }

    #[test]
    fn test_asyncapi_consumption_policy_while() {
        let json = r#"{"while": "${ .counter < 10 }"}"#;
        let policy: AsyncApiMessageConsumptionPolicy = serde_json::from_str(json).unwrap();
        match policy {
            AsyncApiMessageConsumptionPolicy::While { while_ } => {
                assert_eq!(while_, "${ .counter < 10 }");
            }
            _ => panic!("Expected While variant"),
        }
    }

    #[test]
    fn test_grpc_service_with_authentication() {
        let json = r#"{
            "name": "UserService",
            "host": "example.com",
            "port": 50051,
            "authentication": {"use": "grpc-auth"}
        }"#;
        let service: GRPCServiceDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(service.name, "UserService");
        assert!(service.authentication.is_some());
    }

    // Additional tests matching Go SDK's task_call_test.go

    #[test]
    fn test_call_http_with_common_fields() {
        // Matches Go SDK's TestCallHTTP_UnmarshalJSON with full TaskBase fields
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "metadata": {"meta": "data"},
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": "http://example.com",
                "headers": {"Authorization": "Bearer token"},
                "query": {"q": "search"},
                "output": "content",
                "redirect": true
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(http.call, "http");
        assert_eq!(http.common.if_, Some("${condition}".to_string()));
        assert!(http.common.input.is_some());
        assert!(http.common.output.is_some());
        assert!(http.common.timeout.is_some());
        assert_eq!(http.common.then, Some("continue".to_string()));
        assert!(http.common.metadata.is_some());
        assert_eq!(http.with.method, "GET");
        assert_eq!(http.with.output, Some("content".to_string()));
        assert_eq!(http.with.redirect, Some(true));
    }

    #[test]
    fn test_call_http_with_common_fields_roundtrip() {
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": "http://example.com"
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&http).unwrap();
        let deserialized: CallHTTPDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(http, deserialized);
    }

    #[test]
    fn test_call_openapi_with_authentication() {
        // Matches Go SDK's TestCallOpenAPI with authentication.use
        let json = r#"{
            "call": "openapi",
            "with": {
                "document": {
                    "name": "MyOpenAPIDoc",
                    "endpoint": "http://example.com/openapi.json"
                },
                "operationId": "getUsers",
                "parameters": {"param1": "value1", "param2": "value2"},
                "authentication": {"use": "my-auth"},
                "output": "content",
                "redirect": true
            }
        }"#;
        let openapi: CallOpenAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(openapi.with.operation_id, "getUsers");
        assert!(openapi.with.authentication.is_some());
        assert_eq!(openapi.with.output, Some("content".to_string()));
        assert_eq!(openapi.with.redirect, Some(true));
    }

    #[test]
    fn test_call_grpc_with_full_arguments() {
        // Matches Go SDK's TestCallGRPC_UnmarshalJSON
        let json = r#"{
            "call": "grpc",
            "with": {
                "proto": {
                    "name": "MyProtoFile",
                    "endpoint": "http://example.com/protofile"
                },
                "service": {
                    "name": "UserService",
                    "host": "example.com",
                    "port": 50051
                },
                "method": "GetUser",
                "arguments": {"userId": "12345"}
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(grpc.call, "grpc");
        assert_eq!(grpc.with.proto.name, Some("MyProtoFile".to_string()));
        assert_eq!(grpc.with.service.name, "UserService");
        assert_eq!(grpc.with.service.host, "example.com");
        assert_eq!(grpc.with.service.port, Some(50051));
        assert_eq!(grpc.with.method, "GetUser");
        assert!(grpc.with.arguments.is_some());
    }

    #[test]
    fn test_call_asyncapi_with_subscription() {
        let json = r#"{
            "call": "asyncapi",
            "with": {
                "document": {
                    "name": "MyAsyncAPIDoc",
                    "endpoint": "http://example.com/asyncapi.json"
                },
                "operation": "user.signup",
                "server": {"name": "default-server"},
                "protocol": "http",
                "subscription": {
                    "filter": "${ .type == \"order\" }",
                    "consume": {"amount": 5}
                },
                "authentication": {"use": "asyncapi-auth-policy"}
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(asyncapi.with.operation, Some("user.signup".to_string()));
        assert!(asyncapi.with.subscription.is_some());
        let sub = asyncapi.with.subscription.as_ref().unwrap();
        assert!(sub.filter.is_some());
        match &sub.consume {
            AsyncApiMessageConsumptionPolicy::Amount { amount } => assert_eq!(*amount, 5),
            _ => panic!("Expected Amount variant"),
        }
    }

    #[test]
    fn test_call_asyncapi_with_message_and_subscription() {
        // Both message (outbound) and subscription (inbound) in same call
        let json = r#"{
            "call": "asyncapi",
            "with": {
                "document": {"endpoint": "http://example.com/asyncapi.json"},
                "operation": "order.process",
                "protocol": "kafka",
                "message": {"payload": {"orderId": "123"}},
                "subscription": {
                    "consume": {"while": "${ .status != \"completed\" }"}
                }
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        assert!(asyncapi.with.message.is_some());
        assert!(asyncapi.with.subscription.is_some());
        let sub = asyncapi.with.subscription.as_ref().unwrap();
        match &sub.consume {
            AsyncApiMessageConsumptionPolicy::While { while_ } => {
                assert_eq!(while_, "${ .status != \"completed\" }");
            }
            _ => panic!("Expected While variant"),
        }
    }

    #[test]
    fn test_call_function_with_common_fields_roundtrip() {
        let json = r#"{
            "call": "myFunction",
            "with": {"param1": "value1", "param2": 42}
        }"#;
        let func: CallFunctionDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&func).unwrap();
        let deserialized: CallFunctionDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(func, deserialized);
    }

    #[test]
    fn test_call_a2a_with_agent_card() {
        let json = r#"{
            "call": "a2a",
            "with": {
                "agentCard": {
                    "name": "my-agent",
                    "endpoint": "http://example.com/agent-card"
                },
                "server": "http://example.com/a2a-server",
                "method": "tasks/get",
                "parameters": {"taskId": "123"}
            }
        }"#;
        let a2a: CallA2ADefinition = serde_json::from_str(json).unwrap();
        assert_eq!(a2a.call, "a2a");
        assert!(a2a.with.agent_card.is_some());
        assert!(a2a.with.server.is_some());
        assert_eq!(a2a.with.method, "tasks/get");
        assert!(a2a.with.parameters.is_some());
    }

    #[test]
    fn test_call_a2a_roundtrip() {
        let json = r#"{
            "call": "a2a",
            "with": {
                "method": "message/send",
                "parameters": {"message": "hello"}
            }
        }"#;
        let a2a: CallA2ADefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&a2a).unwrap();
        let deserialized: CallA2ADefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(a2a, deserialized);
    }

    // Tests matching Go SDK's task_call_test.go - Call tasks with full TaskBase fields

    #[test]
    fn test_call_grpc_with_common_fields() {
        // Matches Go SDK's TestCallGRPC with full TaskBase fields
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "metadata": {"meta": "data"},
            "call": "grpc",
            "with": {
                "proto": {
                    "name": "MyProtoFile",
                    "endpoint": "http://example.com/protofile"
                },
                "service": {
                    "name": "UserService",
                    "host": "example.com",
                    "port": 50051
                },
                "method": "GetUser",
                "arguments": {"userId": "12345"}
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(grpc.call, "grpc");
        assert_eq!(grpc.common.if_, Some("${condition}".to_string()));
        assert!(grpc.common.input.is_some());
        assert!(grpc.common.output.is_some());
        assert!(grpc.common.timeout.is_some());
        assert_eq!(grpc.common.then, Some("continue".to_string()));
        assert!(grpc.common.metadata.is_some());
        assert_eq!(grpc.with.service.name, "UserService");
        assert_eq!(grpc.with.method, "GetUser");
    }

    #[test]
    fn test_call_grpc_with_common_fields_roundtrip() {
        let json = r#"{
            "if": "${condition}",
            "output": {"as": {"result": "output"}},
            "then": "continue",
            "call": "grpc",
            "with": {
                "proto": {"endpoint": "http://example.com/proto"},
                "service": {"name": "Svc", "host": "example.com"},
                "method": "Get"
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&grpc).unwrap();
        let deserialized: CallGRPCDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(grpc, deserialized);
    }

    #[test]
    fn test_call_openapi_with_common_fields() {
        // Matches Go SDK's TestCallOpenAPI with full TaskBase fields
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "metadata": {"meta": "data"},
            "call": "openapi",
            "with": {
                "document": {
                    "name": "MyOpenAPIDoc",
                    "endpoint": "http://example.com/openapi.json"
                },
                "operationId": "getUsers",
                "parameters": {"param1": "value1"},
                "authentication": {"use": "my-auth"},
                "output": "content",
                "redirect": true
            }
        }"#;
        let openapi: CallOpenAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(openapi.call, "openapi");
        assert_eq!(openapi.common.if_, Some("${condition}".to_string()));
        assert!(openapi.common.input.is_some());
        assert!(openapi.common.output.is_some());
        assert!(openapi.common.timeout.is_some());
        assert_eq!(openapi.common.then, Some("continue".to_string()));
        assert!(openapi.common.metadata.is_some());
        assert_eq!(openapi.with.operation_id, "getUsers");
        assert_eq!(openapi.with.output, Some("content".to_string()));
        assert_eq!(openapi.with.redirect, Some(true));
    }

    #[test]
    fn test_call_openapi_with_common_fields_roundtrip() {
        let json = r#"{
            "if": "${condition}",
            "output": {"as": {"result": "output"}},
            "then": "continue",
            "call": "openapi",
            "with": {
                "document": {"endpoint": "http://example.com/openapi.json"},
                "operationId": "op1"
            }
        }"#;
        let openapi: CallOpenAPIDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&openapi).unwrap();
        let deserialized: CallOpenAPIDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(openapi, deserialized);
    }

    #[test]
    fn test_call_asyncapi_with_common_fields() {
        // Matches Go SDK's TestCallAsyncAPI with full TaskBase fields
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "metadata": {"meta": "data"},
            "call": "asyncapi",
            "with": {
                "document": {
                    "name": "MyAsyncAPIDoc",
                    "endpoint": "http://example.com/asyncapi.json"
                },
                "operation": "user.signup",
                "server": {"name": "default-server"},
                "protocol": "http",
                "message": {
                    "payload": {"userId": "12345"}
                },
                "authentication": {"use": "asyncapi-auth-policy"}
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(asyncapi.call, "asyncapi");
        assert_eq!(asyncapi.common.if_, Some("${condition}".to_string()));
        assert!(asyncapi.common.input.is_some());
        assert!(asyncapi.common.output.is_some());
        assert!(asyncapi.common.timeout.is_some());
        assert_eq!(asyncapi.common.then, Some("continue".to_string()));
        assert!(asyncapi.common.metadata.is_some());
        assert_eq!(asyncapi.with.operation, Some("user.signup".to_string()));
        assert_eq!(asyncapi.with.protocol, Some("http".to_string()));
    }

    #[test]
    fn test_call_asyncapi_with_common_fields_roundtrip() {
        let json = r#"{
            "if": "${condition}",
            "output": {"as": {"result": "output"}},
            "then": "continue",
            "call": "asyncapi",
            "with": {
                "document": {"endpoint": "http://example.com/asyncapi.json"},
                "operation": "user.signup",
                "protocol": "http"
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&asyncapi).unwrap();
        let deserialized: CallAsyncAPIDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(asyncapi, deserialized);
    }

    #[test]
    fn test_call_function_with_common_fields() {
        // Matches Go SDK's TestCallFunction with full TaskBase fields
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "metadata": {"meta": "data"},
            "call": "myFunction",
            "with": {"param1": "value1", "param2": 42}
        }"#;
        let func: CallFunctionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(func.call, "myFunction");
        assert_eq!(func.common.if_, Some("${condition}".to_string()));
        assert!(func.common.input.is_some());
        assert!(func.common.output.is_some());
        assert!(func.common.timeout.is_some());
        assert_eq!(func.common.then, Some("continue".to_string()));
        assert!(func.common.metadata.is_some());
    }

    #[test]
    fn test_call_a2a_with_common_fields() {
        let json = r#"{
            "if": "${condition}",
            "input": {"from": {"key": "value"}},
            "output": {"as": {"result": "output"}},
            "timeout": {"after": "PT10S"},
            "then": "continue",
            "call": "a2a",
            "with": {
                "method": "message/send",
                "parameters": {"message": "hello"}
            }
        }"#;
        let a2a: CallA2ADefinition = serde_json::from_str(json).unwrap();
        assert_eq!(a2a.call, "a2a");
        assert_eq!(a2a.common.if_, Some("${condition}".to_string()));
        assert!(a2a.common.input.is_some());
        assert!(a2a.common.output.is_some());
        assert!(a2a.common.timeout.is_some());
        assert_eq!(a2a.common.then, Some("continue".to_string()));
    }

    #[test]
    fn test_call_http_with_body() {
        // Test HTTP call with body payload
        let json = r#"{
            "call": "http",
            "with": {
                "method": "POST",
                "endpoint": "http://example.com/api",
                "headers": {"Content-Type": "application/json"},
                "body": {"name": "test", "value": 42},
                "output": "content"
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(http.with.method, "POST");
        assert!(http.with.body.is_some());
        let body = http.with.body.unwrap();
        assert_eq!(body["name"], "test");
        assert_eq!(body["value"], 42);
    }

    #[test]
    fn test_call_http_headers_expression() {
        // Test HTTP headers as runtime expression
        let json = r#"{
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": "http://example.com",
                "headers": "${ .requestHeaders }"
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        match &http.with.headers {
            Some(OneOfHeadersOrExpression::Expression(expr)) => {
                assert_eq!(expr, "${ .requestHeaders }");
            }
            _ => panic!("Expected Expression variant for headers"),
        }
    }

    #[test]
    fn test_call_http_query_expression() {
        // Test HTTP query as runtime expression
        let json = r#"{
            "call": "http",
            "with": {
                "method": "GET",
                "endpoint": "http://example.com",
                "query": "${ .queryParams }"
            }
        }"#;
        let http: CallHTTPDefinition = serde_json::from_str(json).unwrap();
        match &http.with.query {
            Some(OneOfQueryOrExpression::Expression(expr)) => {
                assert_eq!(expr, "${ .queryParams }");
            }
            _ => panic!("Expected Expression variant for query"),
        }
    }

    #[test]
    fn test_call_grpc_service_with_authentication_roundtrip() {
        let json = r#"{
            "call": "grpc",
            "with": {
                "proto": {"endpoint": "http://example.com/proto"},
                "service": {
                    "name": "Svc",
                    "host": "example.com",
                    "authentication": {"use": "grpc-auth"}
                },
                "method": "Get"
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&grpc).unwrap();
        let deserialized: CallGRPCDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(grpc, deserialized);
    }

    #[test]
    fn test_call_a2a_with_parameters_expression() {
        // A2A parameters as runtime expression
        let json = r#"{
            "call": "a2a",
            "with": {
                "method": "tasks/get",
                "parameters": "${ .taskParams }"
            }
        }"#;
        let a2a: CallA2ADefinition = serde_json::from_str(json).unwrap();
        match &a2a.with.parameters {
            Some(OneOfA2AParametersOrExpression::Expression(expr)) => {
                assert_eq!(expr, "${ .taskParams }");
            }
            _ => panic!("Expected Expression variant for parameters"),
        }
    }

    #[test]
    fn test_call_grpc_with_authentication() {
        // Matches Go SDK's TestCallGRPC_UnmarshalJSON - GRPCArguments-level authentication
        let json = r#"{
            "call": "grpc",
            "with": {
                "proto": {"endpoint": "http://example.com/proto"},
                "service": {
                    "name": "UserService",
                    "host": "example.com"
                },
                "method": "GetUser",
                "arguments": {"userId": "12345"},
                "authentication": {"use": "my-auth"}
            }
        }"#;
        let grpc: CallGRPCDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(grpc.call, "grpc");
        assert!(grpc.with.authentication.is_some());
        match grpc.with.authentication.unwrap() {
            ReferenceableAuthenticationPolicy::Reference(r) => {
                assert_eq!(r.use_, "my-auth");
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn test_call_asyncapi_channel() {
        // AsyncAPI with channel (v2.6.0)
        let json = r#"{
            "call": "asyncapi",
            "with": {
                "document": {"endpoint": "http://example.com/asyncapi.json"},
                "channel": "user-events",
                "protocol": "kafka",
                "message": {"payload": {"event": "created"}}
            }
        }"#;
        let asyncapi: CallAsyncAPIDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(asyncapi.with.channel, Some("user-events".to_string()));
        assert!(asyncapi.with.message.is_some());
    }
}
