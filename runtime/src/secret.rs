use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// Trait for providing secrets to workflow expressions ($secret variable)
///
/// Implement this trait to provide secret values that can be accessed
/// via the `$secret` expression variable in workflow definitions.
///
/// # Example
///
/// ```
/// use serverless_workflow_runtime::secret::SecretManager;
/// use serde_json::Value;
/// use std::collections::HashMap;
///
/// struct EnvSecretManager;
///
/// impl SecretManager for EnvSecretManager {
///     fn get_secret(&self, key: &str) -> Option<Value> {
///         std::env::var(key)
///             .ok()
///             .map(|v| Value::String(v))
///     }
///
///     fn get_all_secrets(&self) -> Value {
///         let mut map = serde_json::Map::new();
///         for (key, value) in std::env::vars() {
///             map.insert(key, Value::String(value));
///         }
///         Value::Object(map)
///     }
/// }
/// ```
pub trait SecretManager: Send + Sync {
    /// Gets a secret value by key path (e.g., "superman.name")
    fn get_secret(&self, key: &str) -> Option<Value>;

    /// Gets all available secrets as a JSON value
    ///
    /// The returned value should be a JSON object that can be navigated
    /// using dot notation in expressions (e.g., `$secret.superman.name`)
    fn get_all_secrets(&self) -> Value;
}

/// A simple secret manager that stores secrets in a HashMap
#[derive(Debug, Default)]
pub struct MapSecretManager {
    secrets: HashMap<String, Value>,
    cached_all: Mutex<Option<Value>>,
}

impl MapSecretManager {
    /// Creates a new empty MapSecretManager
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a secret key-value pair
    pub fn with_secret(mut self, key: impl Into<String>, value: Value) -> Self {
        self.secrets.insert(key.into(), value);
        *self.cached_all.get_mut().unwrap_or_else(|e| e.into_inner()) = None;
        self
    }

    /// Sets a secret value
    pub fn set_secret(&mut self, key: impl Into<String>, value: Value) {
        self.secrets.insert(key.into(), value);
        *self.cached_all.get_mut().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

impl SecretManager for MapSecretManager {
    fn get_secret(&self, key: &str) -> Option<Value> {
        // Support dot-notation key lookup (e.g., "superman.name" -> nested access)
        let parts: Vec<&str> = key.split('.').collect();
        let first = self.secrets.get(parts[0])?;
        if parts.len() == 1 {
            return Some(first.clone());
        }
        let mut current = first;
        for part in &parts[1..] {
            match current {
                Value::Object(map) => {
                    current = map.get(*part)?;
                }
                _ => return None,
            }
        }
        Some(current.clone())
    }

    fn get_all_secrets(&self) -> Value {
        let mut cache = self.cached_all.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref cached) = *cache {
            return cached.clone();
        }
        let mut map = serde_json::Map::new();
        for (key, value) in &self.secrets {
            map.insert(key.clone(), value.clone());
        }
        let result = Value::Object(map);
        *cache = Some(result.clone());
        result
    }
}

/// A secret manager that reads from environment variables
#[derive(Debug, Default)]
pub struct EnvSecretManager {
    prefix: Option<String>,
    cached_all: Mutex<Option<Value>>,
}

impl EnvSecretManager {
    /// Creates a new EnvSecretManager without prefix
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new EnvSecretManager with a prefix filter
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            cached_all: Mutex::new(None),
        }
    }
}

impl SecretManager for EnvSecretManager {
    fn get_secret(&self, key: &str) -> Option<Value> {
        std::env::var(key).ok().map(Value::String)
    }

    fn get_all_secrets(&self) -> Value {
        let mut cache = self.cached_all.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref cached) = *cache {
            return cached.clone();
        }
        let mut map = serde_json::Map::new();
        for (key, value) in std::env::vars() {
            if let Some(ref prefix) = self.prefix {
                if key.starts_with(prefix) {
                    map.insert(key, Value::String(value));
                }
            } else {
                map.insert(key, Value::String(value));
            }
        }
        let result = Value::Object(map);
        *cache = Some(result.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_map_secret_manager_simple() {
        let mgr = MapSecretManager::new().with_secret("api_key", json!("secret123"));

        assert_eq!(mgr.get_secret("api_key"), Some(json!("secret123")));
        assert_eq!(mgr.get_secret("nonexistent"), None);
    }

    #[test]
    fn test_map_secret_manager_nested() {
        let mgr = MapSecretManager::new().with_secret(
            "superman",
            json!({
                "name": "ClarkKent",
                "enemy": {
                    "name": "Lex Luthor",
                    "isHuman": true
                }
            }),
        );

        assert_eq!(mgr.get_secret("superman.name"), Some(json!("ClarkKent")));
        assert_eq!(
            mgr.get_secret("superman.enemy.name"),
            Some(json!("Lex Luthor"))
        );
        assert_eq!(mgr.get_secret("superman.enemy.isHuman"), Some(json!(true)));
    }

    #[test]
    fn test_map_secret_manager_get_all() {
        let mgr = MapSecretManager::new()
            .with_secret("key1", json!("value1"))
            .with_secret("key2", json!("value2"));

        let all = mgr.get_all_secrets();
        assert_eq!(all["key1"], json!("value1"));
        assert_eq!(all["key2"], json!("value2"));
    }
}
