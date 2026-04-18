use crate::models::task::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the definition of an extension
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionDefinition {
    /// Gets/sets the type of task to extend
    #[serde(rename = "extend")]
    pub extend: String,

    /// Gets/sets a runtime expression, if any, used to determine whether or not the extension should apply in the specified context
    #[serde(rename = "when", skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// Gets/sets a name/definition list, if any, of the tasks to execute before the extended task
    #[serde(rename = "before", skip_serializing_if = "Option::is_none")]
    pub before: Option<Vec<HashMap<String, TaskDefinition>>>,

    /// Gets/sets a name/definition list, if any, of the tasks to execute after the extended task
    #[serde(rename = "after", skip_serializing_if = "Option::is_none")]
    pub after: Option<Vec<HashMap<String, TaskDefinition>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_deserialize_with_before() {
        let json = r#"{
            "extend": "call",
            "when": "${ .shouldExtend }",
            "before": [{"preTask": {"set": {"preKey": "preValue"}}}]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "call");
        assert_eq!(ext.when, Some("${ .shouldExtend }".to_string()));
        assert!(ext.before.is_some());
        assert!(ext.after.is_none());
    }

    #[test]
    fn test_extension_deserialize_with_after() {
        let json = r#"{
            "extend": "set",
            "after": [{"postTask": {"set": {"postKey": "postValue"}}}]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "set");
        assert!(ext.before.is_none());
        assert!(ext.after.is_some());
    }

    #[test]
    fn test_extension_deserialize_extend_all() {
        let json = r#"{
            "extend": "all",
            "before": [{"log": {"set": {"logMsg": "before"}}}],
            "after": [{"logAfter": {"set": {"logMsg": "after"}}}]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "all");
        assert!(ext.before.is_some());
        assert!(ext.after.is_some());
    }

    #[test]
    fn test_extension_roundtrip() {
        let json = r#"{
            "extend": "call",
            "when": "${ .enabled }",
            "before": [{"preTask": {"set": {"key": "val"}}}]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&ext).unwrap();
        let deserialized: ExtensionDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ext, deserialized);
    }

    #[test]
    fn test_extension_minimal() {
        let json = r#"{"extend": "for"}"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "for");
        assert!(ext.when.is_none());
        assert!(ext.before.is_none());
        assert!(ext.after.is_none());
    }

    // Additional tests matching Go SDK's extension_test.go patterns

    #[test]
    fn test_extension_with_call_http_before() {
        // Matches Go SDK's TestExtension_UnmarshalJSON - extension with HTTP call task in before
        let json = r#"{
            "extend": "call",
            "when": "${condition}",
            "before": [
                {"task1": {"call": "http", "with": {"method": "GET", "endpoint": {"uri": "http://example.com"}}}}
            ]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "call");
        assert_eq!(ext.when, Some("${condition}".to_string()));
        assert!(ext.before.is_some());
        let before = ext.before.unwrap();
        assert_eq!(before.len(), 1);
        assert!(before[0].contains_key("task1"));
    }

    #[test]
    fn test_extension_with_before_and_after() {
        // Matches Go SDK's TestExtension_MarshalJSON pattern with both before and after
        let json = r#"{
            "extend": "call",
            "when": "${condition}",
            "before": [
                {"task1": {"set": {"key1": "value1"}}}
            ],
            "after": [
                {"task2": {"set": {"key2": "value2"}}}
            ]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "call");
        assert_eq!(ext.when, Some("${condition}".to_string()));
        assert!(ext.before.is_some());
        assert!(ext.after.is_some());
        let before = ext.before.unwrap();
        let after = ext.after.unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 1);
    }

    #[test]
    fn test_extension_roundtrip_with_before_and_after() {
        let json = r#"{
            "extend": "call",
            "when": "${condition}",
            "before": [
                {"preTask": {"set": {"preKey": "preValue"}}}
            ],
            "after": [
                {"postTask": {"set": {"postKey": "postValue"}}}
            ]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&ext).unwrap();
        let deserialized: ExtensionDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ext, deserialized);
    }

    #[test]
    fn test_extension_extend_all() {
        // "extend": "all" means apply to all task types
        let json = r#"{
            "extend": "all",
            "before": [{"log": {"set": {"msg": "before"}}}],
            "after": [{"logAfter": {"set": {"msg": "after"}}}]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(ext.extend, "all");
        assert!(ext.before.is_some());
        assert!(ext.after.is_some());
    }

    #[test]
    fn test_extension_with_multiple_before_tasks() {
        let json = r#"{
            "extend": "call",
            "before": [
                {"task1": {"set": {"k1": "v1"}}},
                {"task2": {"set": {"k2": "v2"}}}
            ]
        }"#;
        let ext: ExtensionDefinition = serde_json::from_str(json).unwrap();
        let before = ext.before.unwrap();
        assert_eq!(before.len(), 2);
    }
}
