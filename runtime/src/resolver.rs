use std::sync::Arc;
use swf_core::models::workflow::WorkflowDefinition;

/// Trait for resolving workflow definitions by name at runtime.
///
/// Implement this trait to provide dynamic workflow resolution for
/// `run: workflow` tasks. This enables declarative workflow references
/// without pre-registering every sub-workflow via `with_sub_workflow()`.
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
/// use std::collections::HashMap;
/// use swf_runtime::WorkflowResolver;
/// use swf_core::models::workflow::WorkflowDefinition;
///
/// struct MapResolver {
///     workflows: HashMap<String, WorkflowDefinition>,
/// }
///
/// impl WorkflowResolver for MapResolver {
///     fn resolve(&self, name: &str) -> Option<WorkflowDefinition> {
///         self.workflows.get(name).cloned()
///     }
/// }
/// ```
pub trait WorkflowResolver: Send + Sync {
    /// Resolves a workflow definition by name.
    ///
    /// Returns `None` if the workflow is not found. The runtime will
    /// produce an error at task execution time, not at workflow startup.
    fn resolve(&self, name: &str) -> Option<WorkflowDefinition>;
}

/// A simple in-memory workflow resolver backed by a `HashMap`.
///
/// Useful for testing or when all sub-workflows are known at construction time.
pub struct MapWorkflowResolver {
    workflows: std::collections::HashMap<String, WorkflowDefinition>,
}

impl MapWorkflowResolver {
    /// Creates a new empty resolver
    pub fn new() -> Self {
        Self {
            workflows: std::collections::HashMap::new(),
        }
    }

    /// Adds a workflow definition to the resolver
    pub fn add(&mut self, name: impl Into<String>, workflow: WorkflowDefinition) {
        self.workflows.insert(name.into(), workflow);
    }
}

impl Default for MapWorkflowResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowResolver for MapWorkflowResolver {
    fn resolve(&self, name: &str) -> Option<WorkflowDefinition> {
        self.workflows.get(name).cloned()
    }
}

/// A caching wrapper around any [`WorkflowResolver`] that stores resolved definitions.
///
/// Resolved `Some(WorkflowDefinition)` values are cached indefinitely.
/// `None` results (not found) are not cached, allowing retry on subsequent lookups.
pub struct CachingWorkflowResolver {
    inner: Arc<dyn WorkflowResolver>,
    cache: std::sync::Mutex<std::collections::HashMap<String, Option<WorkflowDefinition>>>,
}

impl CachingWorkflowResolver {
    /// Creates a new caching resolver wrapping the given inner resolver
    pub fn new(inner: Arc<dyn WorkflowResolver>) -> Self {
        Self {
            inner,
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Clears the cache, forcing all lookups to go through the inner resolver again
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

impl WorkflowResolver for CachingWorkflowResolver {
    fn resolve(&self, name: &str) -> Option<WorkflowDefinition> {
        // Check cache first (only cache successful resolves)
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cached) = cache.get(name) {
                return cached.clone();
            }
        }

        // Resolve from inner
        let result = self.inner.resolve(name);

        // Cache only Some results (don't cache "not found" to allow retry)
        if result.is_some() {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.insert(name.to_string(), result.clone());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swf_core::models::workflow::{WorkflowDefinition, WorkflowDefinitionMetadata};

    fn test_workflow(name: &str) -> WorkflowDefinition {
        WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default", name, "1.0.0", None, None, None,
        ))
    }

    #[test]
    fn test_map_resolver_found() {
        let mut resolver = MapWorkflowResolver::new();
        resolver.add("test-wf", test_workflow("test-wf"));

        let result = resolver.resolve("test-wf");
        assert!(result.is_some());
    }

    #[test]
    fn test_map_resolver_not_found() {
        let resolver = MapWorkflowResolver::new();
        let result = resolver.resolve("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_caching_resolver_caches_success() {
        let mut inner = MapWorkflowResolver::new();
        inner.add("cached-wf", test_workflow("cached-wf"));

        let caching = CachingWorkflowResolver::new(Arc::new(inner));

        // First call resolves from inner
        let result1 = caching.resolve("cached-wf");
        assert!(result1.is_some());

        // Second call hits cache (inner is still there, but cache is used)
        let result2 = caching.resolve("cached-wf");
        assert!(result2.is_some());
    }

    #[test]
    fn test_caching_resolver_does_not_cache_not_found() {
        let inner = MapWorkflowResolver::new();
        let caching = CachingWorkflowResolver::new(Arc::new(inner));

        // Not found — should not be cached
        let result1 = caching.resolve("missing");
        assert!(result1.is_none());

        // Still tries inner again (if inner were to gain the workflow, it would be found)
        let result2 = caching.resolve("missing");
        assert!(result2.is_none());
    }

    #[test]
    fn test_caching_resolver_clear_cache() {
        let mut inner = MapWorkflowResolver::new();
        inner.add("clear-wf", test_workflow("clear-wf"));

        let caching = CachingWorkflowResolver::new(Arc::new(inner));

        let _ = caching.resolve("clear-wf");
        caching.clear_cache();

        // After clear, cache is empty but inner still has the workflow
        let result = caching.resolve("clear-wf");
        assert!(result.is_some());
    }
}
