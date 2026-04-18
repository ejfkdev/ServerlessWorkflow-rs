use super::*;

// ============== CallFunctionTaskDefinitionBuilder ==============
/// Builder for constructing a call task that invokes a function.
pub struct CallFunctionTaskDefinitionBuilder {
    call_function: CallFunctionDefinition,
}

impl CallFunctionTaskDefinitionBuilder {
    pub fn new(function: &str) -> Self {
        Self {
            call_function: CallFunctionDefinition {
                call: function.to_string(),
                with: None,
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Adds a single named argument to the function call.
    pub fn with(&mut self, name: &str, value: Value) -> &mut Self {
        self.call_function
            .with
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// Replaces all function call arguments with the provided map.
    pub fn with_arguments(&mut self, arguments: HashMap<String, Value>) -> &mut Self {
        self.call_function.with = Some(arguments);
        self
    }
}

impl_task_definition_builder_base!(CallFunctionTaskDefinitionBuilder, call_function, |v| {
    TaskDefinition::Call(Box::new(CallTaskDefinition::Function(v)))
});

// ============== CallHTTPDefinitionBuilder ==============
/// Builder for constructing an HTTP call task.
pub struct CallHTTPDefinitionBuilder {
    task: CallHTTPDefinition,
}

impl CallHTTPDefinitionBuilder {
    pub fn new(method: &str, endpoint: &str) -> Self {
        Self {
            task: CallHTTPDefinition {
                call: "http".to_string(),
                with: HTTPArguments {
                    method: method.to_string(),
                    endpoint: OneOfEndpointDefinitionOrUri::Uri(endpoint.to_string()),
                    headers: None,
                    body: None,
                    query: None,
                    output: None,
                    redirect: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Adds an HTTP header to the request.
    pub fn with_header(&mut self, name: &str, value: &str) -> &mut Self {
        if let OneOfHeadersOrExpression::Map(headers) = self
            .task
            .with
            .headers
            .get_or_insert_with(|| OneOfHeadersOrExpression::Map(HashMap::new()))
        {
            headers.insert(name.to_string(), value.to_string());
        }
        self
    }

    /// Sets the HTTP request body.
    pub fn with_body(&mut self, body: Value) -> &mut Self {
        self.task.with.body = Some(body);
        self
    }

    /// Adds a query parameter to the request.
    pub fn with_query(&mut self, name: &str, value: &str) -> &mut Self {
        if let OneOfQueryOrExpression::Map(query) = self
            .task
            .with
            .query
            .get_or_insert_with(|| OneOfQueryOrExpression::Map(HashMap::new()))
        {
            query.insert(name.to_string(), value.to_string());
        }
        self
    }

    /// Adds multiple query parameters from a map.
    pub fn with_query_map(&mut self, map: HashMap<String, String>) -> &mut Self {
        if let OneOfQueryOrExpression::Map(query) = self
            .task
            .with
            .query
            .get_or_insert_with(|| OneOfQueryOrExpression::Map(HashMap::new()))
        {
            query.extend(map);
        }
        self
    }

    /// Sets query parameters from a runtime expression (e.g., `"${ .queryParams }"`).
    pub fn with_query_expression(&mut self, expr: &str) -> &mut Self {
        self.task.with.query = Some(OneOfQueryOrExpression::Expression(expr.to_string()));
        self
    }

    /// Sets the output format for the HTTP response.
    pub fn with_output_format(&mut self, output: &str) -> &mut Self {
        self.task.with.output = Some(output.to_string());
        self
    }

    /// Sets whether HTTP redirects should be followed.
    pub fn with_redirect(&mut self, redirect: bool) -> &mut Self {
        self.task.with.redirect = Some(redirect);
        self
    }
}

impl_task_definition_builder_base!(CallHTTPDefinitionBuilder, task, |v| TaskDefinition::Call(
    Box::new(CallTaskDefinition::HTTP(v))
));

// ============== CallGRPCDefinitionBuilder ==============
/// Builder for constructing a gRPC call task.
pub struct CallGRPCDefinitionBuilder {
    task: CallGRPCDefinition,
}

impl CallGRPCDefinitionBuilder {
    pub fn new(proto_url: &str, service_name: &str, method: &str) -> Self {
        Self {
            task: CallGRPCDefinition {
                call: "grpc".to_string(),
                with: GRPCArguments {
                    proto: ExternalResourceDefinition {
                        name: None,
                        endpoint: OneOfEndpointDefinitionOrUri::Uri(proto_url.to_string()),
                    },
                    service: GRPCServiceDefinition {
                        name: service_name.to_string(),
                        ..Default::default()
                    },
                    method: method.to_string(),
                    arguments: None,
                    authentication: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Sets the gRPC service host.
    pub fn with_host(&mut self, host: &str) -> &mut Self {
        self.task.with.service.host = host.to_string();
        self
    }

    /// Sets the gRPC service port.
    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.task.with.service.port = Some(port);
        self
    }

    /// Adds a single named argument to the gRPC call.
    pub fn with_argument(&mut self, name: &str, value: Value) -> &mut Self {
        self.task
            .with
            .arguments
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// References a named authentication policy for the gRPC call.
    pub fn with_authentication_use(&mut self, auth_name: &str) -> &mut Self {
        self.task.with.authentication = Some(ReferenceableAuthenticationPolicy::Reference(
            AuthenticationPolicyReference {
                use_: auth_name.to_string(),
            },
        ));
        self
    }
}

impl_task_definition_builder_base!(CallGRPCDefinitionBuilder, task, |v| TaskDefinition::Call(
    Box::new(CallTaskDefinition::GRPC(Box::new(v)))
));

// ============== CallOpenAPIDefinitionBuilder ==============
/// Builder for constructing an OpenAPI call task.
pub struct CallOpenAPIDefinitionBuilder {
    task: CallOpenAPIDefinition,
}

impl CallOpenAPIDefinitionBuilder {
    pub fn new(document_url: &str, operation_id: &str) -> Self {
        Self {
            task: CallOpenAPIDefinition {
                call: "openapi".to_string(),
                with: OpenAPIArguments {
                    document: ExternalResourceDefinition {
                        name: None,
                        endpoint: OneOfEndpointDefinitionOrUri::Uri(document_url.to_string()),
                    },
                    operation_id: operation_id.to_string(),
                    parameters: None,
                    authentication: None,
                    output: None,
                    redirect: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Adds a single named parameter to the OpenAPI call.
    pub fn with_parameter(&mut self, name: &str, value: Value) -> &mut Self {
        self.task
            .with
            .parameters
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// Sets the output format for the OpenAPI response.
    pub fn with_output_format(&mut self, output: &str) -> &mut Self {
        self.task.with.output = Some(output.to_string());
        self
    }
}

impl_task_definition_builder_base!(
    CallOpenAPIDefinitionBuilder,
    task,
    |v| TaskDefinition::Call(Box::new(CallTaskDefinition::OpenAPI(v)))
);

// ============== CallAsyncAPIDefinitionBuilder ==============
/// Builder for constructing an AsyncAPI call task.
pub struct CallAsyncAPIDefinitionBuilder {
    task: CallAsyncAPIDefinition,
}

impl CallAsyncAPIDefinitionBuilder {
    pub fn new(document_url: &str) -> Self {
        Self {
            task: CallAsyncAPIDefinition {
                call: "asyncapi".to_string(),
                with: AsyncApiArguments {
                    document: ExternalResourceDefinition {
                        name: None,
                        endpoint: OneOfEndpointDefinitionOrUri::Uri(document_url.to_string()),
                    },
                    channel: None,
                    operation: None,
                    server: None,
                    protocol: None,
                    message: None,
                    subscription: None,
                    authentication: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Sets the AsyncAPI channel name.
    pub fn with_channel(&mut self, channel: &str) -> &mut Self {
        self.task.with.channel = Some(channel.to_string());
        self
    }

    /// Sets the AsyncAPI operation.
    pub fn with_operation(&mut self, operation: &str) -> &mut Self {
        self.task.with.operation = Some(operation.to_string());
        self
    }

    /// Sets the AsyncAPI protocol (e.g., "kafka", "mqtt").
    pub fn with_protocol(&mut self, protocol: &str) -> &mut Self {
        self.task.with.protocol = Some(protocol.to_string());
        self
    }
}

impl_task_definition_builder_base!(CallAsyncAPIDefinitionBuilder, task, |v| {
    TaskDefinition::Call(Box::new(CallTaskDefinition::AsyncAPI(v)))
});

// ============== CallA2ADefinitionBuilder ==============
/// Builder for constructing an Agent-to-Agent (A2A) call task.
pub struct CallA2ADefinitionBuilder {
    task: CallA2ADefinition,
}

impl CallA2ADefinitionBuilder {
    pub fn new(method: &str) -> Self {
        Self {
            task: CallA2ADefinition {
                call: "a2a".to_string(),
                with: A2AArguments {
                    agent_card: None,
                    server: None,
                    method: method.to_string(),
                    parameters: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Sets the agent card URL for the A2A call.
    pub fn with_agent_card(&mut self, url: &str) -> &mut Self {
        self.task.with.agent_card = Some(ExternalResourceDefinition {
            name: None,
            endpoint: OneOfEndpointDefinitionOrUri::Uri(url.to_string()),
        });
        self
    }

    /// Adds a single named parameter to the A2A call.
    pub fn with_parameter(&mut self, name: &str, value: Value) -> &mut Self {
        if let Some(ref mut params) = self.task.with.parameters {
            if let OneOfA2AParametersOrExpression::Map(ref mut map) = params {
                map.insert(name.to_string(), value);
            }
        } else {
            let mut map = HashMap::new();
            map.insert(name.to_string(), value);
            self.task.with.parameters = Some(OneOfA2AParametersOrExpression::Map(map));
        }
        self
    }
}

impl_task_definition_builder_base!(CallA2ADefinitionBuilder, task, |v| TaskDefinition::Call(
    Box::new(CallTaskDefinition::A2A(v))
));
