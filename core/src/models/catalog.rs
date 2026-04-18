use crate::models::resource::*;
use serde::{Deserialize, Serialize};

/// Represents the definition of a workflow component catalog
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogDefinition {
    /// Gets/sets the endpoint that defines the root URL at which the catalog is located
    #[serde(rename = "endpoint")]
    pub endpoint: OneOfEndpointDefinitionOrUri,
}
