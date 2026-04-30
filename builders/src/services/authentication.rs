use swf_core::models::authentication::*;

/// Represents a service used to build AuthenticationPolicyDefinitions
#[derive(Default)]
pub struct AuthenticationPolicyDefinitionBuilder {
    reference: Option<String>,
    builder: Option<AuthenticationSchemeBuilder>,
}

impl AuthenticationPolicyDefinitionBuilder {
    /// Initializes a new AuthenticationPolicyDefinition
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the name of the top-level authentication policy to use
    pub fn use_(&mut self, reference: &str) -> &mut Self {
        self.reference = Some(reference.to_string());
        self
    }

    /// Configures the policy to use 'Basic' authentication
    pub fn basic(&mut self) -> &mut BasicAuthenticationSchemeDefinitionBuilder {
        let builder = BasicAuthenticationSchemeDefinitionBuilder::new();
        self.builder = Some(AuthenticationSchemeBuilder::Basic(builder));
        if let Some(AuthenticationSchemeBuilder::Basic(ref mut builder)) = self.builder {
            builder
        } else {
            unreachable!("Builder should always be set to Basic");
        }
    }

    /// Configures the policy to use 'Bearer' authentication
    pub fn bearer(&mut self) -> &mut BearerAuthenticationSchemeDefinitionBuilder {
        let builder = BearerAuthenticationSchemeDefinitionBuilder::new();
        self.builder = Some(AuthenticationSchemeBuilder::Bearer(builder));
        if let Some(AuthenticationSchemeBuilder::Bearer(ref mut builder)) = self.builder {
            builder
        } else {
            unreachable!("Builder should always be set to Bearer");
        }
    }

    /// Configures the policy to use 'Digest' authentication
    pub fn digest(&mut self) -> &mut DigestAuthenticationSchemeDefinitionBuilder {
        let builder = DigestAuthenticationSchemeDefinitionBuilder::new();
        self.builder = Some(AuthenticationSchemeBuilder::Digest(builder));
        if let Some(AuthenticationSchemeBuilder::Digest(ref mut builder)) = self.builder {
            builder
        } else {
            unreachable!("Builder should always be set to Digest");
        }
    }

    /// Configures the policy to use 'OAuth2' authentication
    pub fn oauth2(&mut self) -> &mut OAuth2AuthenticationSchemeDefinitionBuilder {
        let builder = OAuth2AuthenticationSchemeDefinitionBuilder::new();
        self.builder = Some(AuthenticationSchemeBuilder::OAuth2(builder));
        if let Some(AuthenticationSchemeBuilder::OAuth2(ref mut builder)) = self.builder {
            builder
        } else {
            unreachable!("Builder should always be set to OAuth2");
        }
    }

    /// Configures the policy to use 'Oidc' authentication
    pub fn oidc(&mut self) -> &mut OidcAuthenticationSchemeDefinitionBuilder {
        let builder = OidcAuthenticationSchemeDefinitionBuilder::new();
        self.builder = Some(AuthenticationSchemeBuilder::Oidc(builder));
        if let Some(AuthenticationSchemeBuilder::Oidc(ref mut builder)) = self.builder {
            builder
        } else {
            unreachable!("Builder should always be set to Oidc");
        }
    }

    /// Builds the configured AuthenticationPolicyDefinition
    pub fn build(self) -> AuthenticationPolicyDefinition {
        if let Some(reference) = self.reference {
            AuthenticationPolicyDefinition {
                use_: Some(reference),
                ..Default::default()
            }
        } else {
            if let Some(builder) = self.builder {
                match builder {
                    AuthenticationSchemeBuilder::Basic(builder) => builder.build(),
                    AuthenticationSchemeBuilder::Bearer(builder) => builder.build(),
                    AuthenticationSchemeBuilder::Digest(builder) => builder.build(),
                    AuthenticationSchemeBuilder::OAuth2(builder) => builder.build(),
                    AuthenticationSchemeBuilder::Oidc(builder) => builder.build(),
                }
            } else {
                panic!("The authentication policy must be configured");
            }
        }
    }
}

/// Enumerates all supported authentication scheme builders
pub(crate) enum AuthenticationSchemeBuilder {
    Basic(BasicAuthenticationSchemeDefinitionBuilder),
    Bearer(BearerAuthenticationSchemeDefinitionBuilder),
    Digest(DigestAuthenticationSchemeDefinitionBuilder),
    OAuth2(OAuth2AuthenticationSchemeDefinitionBuilder),
    Oidc(OidcAuthenticationSchemeDefinitionBuilder),
}

/// Represents the service used to build BasicAuthenticationSchemeDefinitions
#[derive(Default)]
pub struct BasicAuthenticationSchemeDefinitionBuilder {
    scheme: BasicAuthenticationSchemeDefinition,
}

impl BasicAuthenticationSchemeDefinitionBuilder {
    /// Initializes a new BasicAuthenticationSchemeDefinitionBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the authentication scheme to load from the specified secret
    pub fn use_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme.use_ = Some(secret.to_string());
        self
    }

    /// Sets the username to use
    pub fn with_username(&mut self, username: &str) -> &mut Self {
        self.scheme.username = Some(username.to_string());
        self
    }

    /// Sets the password to use
    pub fn with_password(&mut self, password: &str) -> &mut Self {
        self.scheme.password = Some(password.to_string());
        self
    }

    /// Builds the configured AuthenticationPolicyDefinition
    pub fn build(self) -> AuthenticationPolicyDefinition {
        AuthenticationPolicyDefinition {
            basic: Some(self.scheme),
            ..Default::default()
        }
    }
}

/// Represents the service used to build BearerAuthenticationSchemeDefinitions
#[derive(Default)]
pub struct BearerAuthenticationSchemeDefinitionBuilder {
    scheme: BearerAuthenticationSchemeDefinition,
}

impl BearerAuthenticationSchemeDefinitionBuilder {
    /// Initializes a new BearerAuthenticationSchemeDefinitionBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the authentication scheme to load from the specified secret
    pub fn use_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme.use_ = Some(secret.to_string());
        self
    }

    /// Sets the bearer token to use
    pub fn with_token(&mut self, token: &str) -> &mut Self {
        self.scheme.token = Some(token.to_string());
        self
    }

    /// Builds the configured AuthenticationPolicyDefinition
    pub fn build(self) -> AuthenticationPolicyDefinition {
        AuthenticationPolicyDefinition {
            bearer: Some(self.scheme),
            ..Default::default()
        }
    }
}

/// Represents the service used to build DigestAuthenticationSchemeDefinitions
#[derive(Default)]
pub struct DigestAuthenticationSchemeDefinitionBuilder {
    scheme: DigestAuthenticationSchemeDefinition,
}

impl DigestAuthenticationSchemeDefinitionBuilder {
    /// Initializes a new DigestAuthenticationSchemeDefinitionBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the authentication scheme to load from the specified secret
    pub fn use_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme.use_ = Some(secret.to_string());
        self
    }

    /// Sets the username to use
    pub fn with_username(&mut self, username: &str) -> &mut Self {
        self.scheme.username = Some(username.to_string());
        self
    }

    /// Sets the password to use
    pub fn with_password(&mut self, password: &str) -> &mut Self {
        self.scheme.password = Some(password.to_string());
        self
    }

    /// Builds the configured AuthenticationPolicyDefinition
    pub fn build(self) -> AuthenticationPolicyDefinition {
        AuthenticationPolicyDefinition {
            digest: Some(self.scheme),
            ..Default::default()
        }
    }
}

/// Represents the service used to build OAuth2AuthenticationSchemeDefinitions
#[derive(Default)]
pub struct OAuth2AuthenticationSchemeDefinitionBuilder {
    scheme: OAuth2AuthenticationSchemeDefinition,
}

impl OAuth2AuthenticationSchemeDefinitionBuilder {
    /// Initializes a new OAuth2AuthenticationSchemeDefinitionBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the authentication scheme to load from the specified secret
    pub fn use_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme.use_ = Some(secret.to_string());
        self
    }

    /// Sets the OAuth2 authority URL
    pub fn with_authority(&mut self, authority: &str) -> &mut Self {
        self.scheme.authority = Some(authority.to_string());
        self
    }

    /// Sets the OAuth2 grant type
    pub fn with_grant(&mut self, grant: &str) -> &mut Self {
        self.scheme.grant = Some(grant.to_string());
        self
    }

    /// Sets the OAuth2 client id
    pub fn with_client_id(&mut self, id: &str) -> &mut Self {
        self.scheme.client.get_or_insert_with(Default::default).id = Some(id.to_string());
        self
    }

    /// Sets the OAuth2 client secret
    pub fn with_client_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme
            .client
            .get_or_insert_with(Default::default)
            .secret = Some(secret.to_string());
        self
    }

    /// Sets the OAuth2 scopes
    pub fn with_scopes(&mut self, scopes: Vec<String>) -> &mut Self {
        self.scheme.scopes = Some(scopes);
        self
    }

    /// Builds the configured AuthenticationPolicyDefinition
    pub fn build(self) -> AuthenticationPolicyDefinition {
        AuthenticationPolicyDefinition {
            oauth2: Some(self.scheme),
            ..Default::default()
        }
    }
}

/// Represents the service used to build OpenIDConnectSchemeDefinitions
#[derive(Default)]
pub struct OidcAuthenticationSchemeDefinitionBuilder {
    scheme: OpenIDConnectSchemeDefinition,
}

impl OidcAuthenticationSchemeDefinitionBuilder {
    /// Initializes a new OidcAuthenticationSchemeDefinitionBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the authentication scheme to load from the specified secret
    pub fn use_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme.use_ = Some(secret.to_string());
        self
    }

    /// Sets the OIDC authority URL (full token endpoint URL)
    pub fn with_authority(&mut self, authority: &str) -> &mut Self {
        self.scheme.authority = Some(authority.to_string());
        self
    }

    /// Sets the OIDC grant type
    pub fn with_grant(&mut self, grant: &str) -> &mut Self {
        self.scheme.grant = Some(grant.to_string());
        self
    }

    /// Sets the OIDC client id
    pub fn with_client_id(&mut self, id: &str) -> &mut Self {
        self.scheme.client.get_or_insert_with(Default::default).id = Some(id.to_string());
        self
    }

    /// Sets the OIDC client secret
    pub fn with_client_secret(&mut self, secret: &str) -> &mut Self {
        self.scheme
            .client
            .get_or_insert_with(Default::default)
            .secret = Some(secret.to_string());
        self
    }

    /// Sets the OIDC scopes
    pub fn with_scopes(&mut self, scopes: Vec<String>) -> &mut Self {
        self.scheme.scopes = Some(scopes);
        self
    }

    /// Builds the configured AuthenticationPolicyDefinition
    pub fn build(self) -> AuthenticationPolicyDefinition {
        AuthenticationPolicyDefinition {
            oidc: Some(self.scheme),
            ..Default::default()
        }
    }
}
