use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// Represents an ordered key/value map array.
///
/// Internally uses `Vec<(K, V)>` for efficient storage, but serializes to/from
/// the Serverless Workflow DSL format: `[{"key1": "value1"}, {"key2": "value2"}]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map<TKey, TValue>
where
    TKey: Eq + Hash + Clone + PartialEq,
    TValue: Clone + PartialEq,
{
    /// The ordered entries as (key, value) pairs
    pub entries: Vec<(TKey, TValue)>,
}

impl<TKey, TValue> Default for Map<TKey, TValue>
where
    TKey: Eq + Hash + Clone + PartialEq,
    TValue: Clone + PartialEq,
{
    fn default() -> Self {
        Map {
            entries: Vec::new(),
        }
    }
}

impl<TKey, TValue> Map<TKey, TValue>
where
    TKey: Eq + Hash + Clone + PartialEq,
    TValue: Clone + PartialEq,
{
    /// Initializes a new map
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes a new map with the provided entries
    pub fn from(entries: Vec<(TKey, TValue)>) -> Self {
        Map { entries }
    }

    /// Adds the specified entry
    pub fn add(&mut self, key: TKey, value: TValue) {
        self.entries.push((key, value));
    }

    /// Gets the number of entries in the map
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the map is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Checks if the map contains an entry with the specified key
    pub fn contains_key(&self, key: &TKey) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Gets a reference to the value associated with the specified key
    pub fn get(&self, key: &TKey) -> Option<&TValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

// Custom serialization: Vec<(K,V)> -> array of single-key objects [{k: v}, ...]
impl<TKey, TValue> Serialize for Map<TKey, TValue>
where
    TKey: Eq + Hash + Clone + Serialize + PartialEq,
    TValue: Clone + Serialize + PartialEq,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.entries.len()))?;
        for (key, value) in &self.entries {
            let mut hm = HashMap::new();
            hm.insert(key, value);
            seq.serialize_element(&hm)?;
        }
        seq.end()
    }
}

// Custom deserialization: array of single-key objects -> Vec<(K,V)>
impl<'de, TKey, TValue> Deserialize<'de> for Map<TKey, TValue>
where
    TKey: Eq + Hash + Clone + Deserialize<'de> + PartialEq,
    TValue: Clone + Deserialize<'de> + PartialEq,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MapVisitor<TKey, TValue> {
            _phantom: std::marker::PhantomData<(TKey, TValue)>,
        }

        impl<'de, TKey, TValue> Visitor<'de> for MapVisitor<TKey, TValue>
        where
            TKey: Eq + Hash + Clone + Deserialize<'de> + PartialEq,
            TValue: Clone + Deserialize<'de> + PartialEq,
        {
            type Value = Map<TKey, TValue>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an array of single-key objects")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(hm) = seq.next_element::<HashMap<TKey, TValue>>()? {
                    for (k, v) in hm {
                        entries.push((k, v));
                    }
                }
                Ok(Map { entries })
            }
        }

        deserializer.deserialize_seq(MapVisitor {
            _phantom: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_new() {
        let map: Map<String, String> = Map::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_map_default() {
        let map: Map<String, String> = Map::default();
        assert!(map.is_empty());
    }

    #[test]
    fn test_map_from_entries() {
        let map: Map<String, String> = Map::from(vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&"key1".to_string()), Some(&"value1".to_string()));
        assert_eq!(map.get(&"key2".to_string()), Some(&"value2".to_string()));
    }

    #[test]
    fn test_map_add() {
        let mut map: Map<String, String> = Map::new();
        map.add("key1".to_string(), "value1".to_string());
        map.add("key2".to_string(), "value2".to_string());
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&"key1".to_string()), Some(&"value1".to_string()));
    }

    #[test]
    fn test_map_contains_key() {
        let map: Map<String, String> = Map::from(vec![("key1".to_string(), "value1".to_string())]);
        assert!(map.contains_key(&"key1".to_string()));
        assert!(!map.contains_key(&"key2".to_string()));
    }

    #[test]
    fn test_map_get_missing_key() {
        let map: Map<String, String> = Map::from(vec![("key1".to_string(), "value1".to_string())]);
        assert!(map.get(&"nonexistent".to_string()).is_none());
    }

    #[test]
    fn test_map_serde_roundtrip() {
        let map: Map<String, String> = Map::from(vec![
            ("task1".to_string(), "value1".to_string()),
            ("task2".to_string(), "value2".to_string()),
        ]);
        let json = serde_json::to_string(&map).unwrap();
        let deserialized: Map<String, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(map, deserialized);
    }

    #[test]
    fn test_map_serializes_to_array_of_single_key_objects() {
        let map: Map<String, i32> = Map::from(vec![("a".to_string(), 1), ("b".to_string(), 2)]);
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, r#"[{"a":1},{"b":2}]"#);
    }

    #[test]
    fn test_map_deserializes_from_array_of_single_key_objects() {
        let json = r#"[{"a":1},{"b":2}]"#;
        let map: Map<String, i32> = serde_json::from_str(json).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&"a".to_string()), Some(&1));
        assert_eq!(map.get(&"b".to_string()), Some(&2));
    }
}
