use crate::models::duration::*;
use serde::{Deserialize, Serialize};

/// Represents the definition of a timeout
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutDefinition {
    /// Gets/sets the duration after which to timeout
    pub after: OneOfDurationOrIso8601Expression,
}

define_one_of_or_reference!(
    /// Represents a value that can be either a TimeoutDefinition or a reference to a TimeoutDefinition
    OneOfTimeoutDefinitionOrReference, Timeout(TimeoutDefinition)
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_inline_duration() {
        let json = r#"{"after": "PT30S"}"#;
        let timeout: TimeoutDefinition = serde_json::from_str(json).unwrap();
        match timeout.after {
            OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => assert_eq!(expr, "PT30S"),
            _ => panic!("Expected Iso8601Expression variant"),
        }
    }

    #[test]
    fn test_timeout_reference() {
        let json = r#""myTimeout""#;
        let ref_val: OneOfTimeoutDefinitionOrReference = serde_json::from_str(json).unwrap();
        match ref_val {
            OneOfTimeoutDefinitionOrReference::Reference(r) => assert_eq!(r, "myTimeout"),
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn test_timeout_roundtrip() {
        let json = r#"{"after": "PT10M"}"#;
        let timeout: TimeoutDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&timeout).unwrap();
        let deserialized: TimeoutDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(timeout, deserialized);
    }

    // Additional tests matching Go SDK's timeout_test.go

    #[test]
    fn test_timeout_inline_duration_struct() {
        let json = r#"{"after": {"days": 1, "hours": 2}}"#;
        let timeout: TimeoutDefinition = serde_json::from_str(json).unwrap();
        match &timeout.after {
            OneOfDurationOrIso8601Expression::Duration(d) => {
                assert_eq!(d.days, Some(1));
                assert_eq!(d.hours, Some(2));
            }
            _ => panic!("Expected Duration variant"),
        }
    }

    #[test]
    fn test_timeout_iso8601_with_milliseconds() {
        let json = r#"{"after": "PT2S500MS"}"#;
        let timeout: TimeoutDefinition = serde_json::from_str(json).unwrap();
        match &timeout.after {
            OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
                assert_eq!(expr, "PT2S500MS");
            }
            _ => panic!("Expected Iso8601Expression variant"),
        }
    }

    #[test]
    fn test_timeout_reference_roundtrip() {
        let ref_val = OneOfTimeoutDefinitionOrReference::Reference("myTimeout".to_string());
        let serialized = serde_json::to_string(&ref_val).unwrap();
        assert_eq!(serialized, r#""myTimeout""#);
        let deserialized: OneOfTimeoutDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(ref_val, deserialized);
    }

    #[test]
    fn test_timeout_or_reference_inline() {
        let json = r#"{"after": {"days": 1, "hours": 2}}"#;
        let tor: OneOfTimeoutDefinitionOrReference = serde_json::from_str(json).unwrap();
        match tor {
            OneOfTimeoutDefinitionOrReference::Timeout(t) => match t.after {
                OneOfDurationOrIso8601Expression::Duration(d) => {
                    assert_eq!(d.days, Some(1));
                    assert_eq!(d.hours, Some(2));
                }
                _ => panic!("Expected Duration variant"),
            },
            _ => panic!("Expected Timeout variant"),
        }
    }

    #[test]
    fn test_timeout_or_reference_string() {
        let json = r#""some-timeout-reference""#;
        let tor: OneOfTimeoutDefinitionOrReference = serde_json::from_str(json).unwrap();
        match tor {
            OneOfTimeoutDefinitionOrReference::Reference(r) => {
                assert_eq!(r, "some-timeout-reference");
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    // Additional tests matching Go SDK's timeout_test.go

    #[test]
    fn test_timeout_iso8601_full() {
        // Matches Go SDK: Valid ISO 8601 duration
        let json = r#"{"after": "P3DT4H5M6S250MS"}"#;
        let timeout: TimeoutDefinition = serde_json::from_str(json).unwrap();
        match &timeout.after {
            OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
                assert_eq!(expr, "P3DT4H5M6S250MS");
            }
            _ => panic!("Expected Iso8601Expression variant"),
        }
    }

    #[test]
    fn test_timeout_or_reference_roundtrip_inline() {
        // Matches Go SDK: Valid Timeout
        let json = r#"{"after": {"days": 1, "hours": 2}}"#;
        let tor: OneOfTimeoutDefinitionOrReference = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&tor).unwrap();
        let deserialized: OneOfTimeoutDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(tor, deserialized);
    }

    #[test]
    fn test_timeout_or_reference_roundtrip_string_ref() {
        // Matches Go SDK: Valid Ref
        let ref_val =
            OneOfTimeoutDefinitionOrReference::Reference("some-timeout-reference".to_string());
        let serialized = serde_json::to_string(&ref_val).unwrap();
        assert_eq!(serialized, r#""some-timeout-reference""#);
        let deserialized: OneOfTimeoutDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(ref_val, deserialized);
    }

    #[test]
    fn test_timeout_inline_roundtrip() {
        let json = r#"{"after": {"days": 1, "hours": 2, "minutes": 30}}"#;
        let timeout: TimeoutDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&timeout).unwrap();
        let deserialized: TimeoutDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(timeout, deserialized);
    }

    #[test]
    fn test_timeout_default() {
        let timeout = TimeoutDefinition::default();
        match &timeout.after {
            OneOfDurationOrIso8601Expression::Duration(d) => {
                assert_eq!(d.total_milliseconds(), 0);
            }
            _ => panic!("Expected default Duration variant"),
        }
    }

    #[test]
    fn test_oneof_timeout_default() {
        let oneof = OneOfTimeoutDefinitionOrReference::default();
        match oneof {
            OneOfTimeoutDefinitionOrReference::Timeout(_) => {}
            _ => panic!("Expected default Timeout variant"),
        }
    }
}
