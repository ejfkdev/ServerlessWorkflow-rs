use crate::models::expression::is_strict_expr;
use serde::{de, Deserialize, Serialize};
use std::fmt;

/// Represents a duration
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Duration {
    /// Gets/sets the number of days, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u64>,

    /// Gets/sets the number of hours, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours: Option<u64>,

    /// Gets/sets the number of minutes, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u64>,

    /// Gets/sets the number of seconds, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,

    /// Gets/sets the number of milliseconds, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milliseconds: Option<u64>,
}

impl<'de> de::Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DurationVisitor;

        impl<'de> de::Visitor<'de> for DurationVisitor {
            type Value = Duration;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a duration object with at least one property")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut days: Option<u64> = None;
                let mut hours: Option<u64> = None;
                let mut minutes: Option<u64> = None;
                let mut seconds: Option<u64> = None;
                let mut milliseconds: Option<u64> = None;
                let mut has_key = false;

                while let Some(key) = map.next_key::<String>()? {
                    has_key = true;
                    match key.as_str() {
                        "days" => {
                            days = Some(map.next_value()?);
                        }
                        "hours" => {
                            hours = Some(map.next_value()?);
                        }
                        "minutes" => {
                            minutes = Some(map.next_value()?);
                        }
                        "seconds" => {
                            seconds = Some(map.next_value()?);
                        }
                        "milliseconds" => {
                            milliseconds = Some(map.next_value()?);
                        }
                        other => {
                            return Err(de::Error::custom(format!(
                                "unexpected key '{}' in duration object",
                                other
                            )));
                        }
                    }
                }

                if !has_key {
                    return Err(de::Error::custom(
                        "duration object must include at least one property",
                    ));
                }

                Ok(Duration {
                    days,
                    hours,
                    minutes,
                    seconds,
                    milliseconds,
                })
            }
        }

        deserializer.deserialize_map(DurationVisitor)
    }
}
macro_rules! from_unit {
    ($name:ident, $field:ident) => {
        pub fn $name(v: u64) -> Self {
            Self {
                $field: Some(v),
                ..Self::default()
            }
        }
    };
}

macro_rules! total_as {
    ($name:ident, $divisor:expr) => {
        pub fn $name(&self) -> f64 {
            self.total_milliseconds() as f64 / $divisor
        }
    };
}

impl Duration {
    from_unit!(from_days, days);
    from_unit!(from_hours, hours);
    from_unit!(from_minutes, minutes);
    from_unit!(from_seconds, seconds);
    from_unit!(from_milliseconds, milliseconds);

    total_as!(total_days, 24.0 * 60.0 * 60.0 * 1000.0);
    total_as!(total_hours, 60.0 * 60.0 * 1000.0);
    total_as!(total_minutes, 60.0 * 1000.0);
    total_as!(total_seconds, 1000.0);

    /// Gets the the duration's total amount of milliseconds
    pub fn total_milliseconds(&self) -> u64 {
        let total: u128 = (self.days.unwrap_or(0) as u128) * 86_400_000
            + (self.hours.unwrap_or(0) as u128) * 3_600_000
            + (self.minutes.unwrap_or(0) as u128) * 60_000
            + (self.seconds.unwrap_or(0) as u128) * 1_000
            + self.milliseconds.unwrap_or(0) as u128;
        total.try_into().unwrap_or(u64::MAX)
    }
}
impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if let Some(days) = self.days {
            parts.push(format!("{} days", days));
        }
        if let Some(hours) = self.hours {
            parts.push(format!("{} hours", hours));
        }
        if let Some(minutes) = self.minutes {
            parts.push(format!("{} minutes", minutes));
        }
        if let Some(seconds) = self.seconds {
            parts.push(format!("{} seconds", seconds));
        }
        if let Some(milliseconds) = self.milliseconds {
            parts.push(format!("{} milliseconds", milliseconds));
        }
        write!(f, "{}", parts.join(" "))
    }
}

/// Represents a value that can be either a Duration or an ISO 8601 duration expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OneOfDurationOrIso8601Expression {
    /// Variant holding a duration
    Duration(Duration),
    /// Variant holding an ISO 8601 duration expression
    Iso8601Expression(String),
}

impl<'de> de::Deserialize<'de> for OneOfDurationOrIso8601Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Use a helper that can handle both object and string forms
        struct OneOfDurationVisitor;

        impl<'de> de::Visitor<'de> for OneOfDurationVisitor {
            type Value = OneOfDurationOrIso8601Expression;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a duration object or an ISO 8601 duration string")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                // Deserialize as Duration (inline object)
                let duration = Duration::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(OneOfDurationOrIso8601Expression::Duration(duration))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Runtime expressions (e.g., "${ .delay }") are accepted as-is
                if is_strict_expr(v) {
                    return Ok(OneOfDurationOrIso8601Expression::Iso8601Expression(
                        v.to_string(),
                    ));
                }
                // Validate ISO 8601 expression (matches Go SDK's Duration.UnmarshalJSON behavior)
                if !is_iso8601_duration_valid(v) {
                    return Err(de::Error::custom(format!(
                        "invalid ISO 8601 duration expression: '{}'",
                        v
                    )));
                }
                Ok(OneOfDurationOrIso8601Expression::Iso8601Expression(
                    v.to_string(),
                ))
            }
        }

        deserializer.deserialize_any(OneOfDurationVisitor)
    }
}

impl serde::Serialize for OneOfDurationOrIso8601Expression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            OneOfDurationOrIso8601Expression::Duration(d) => d.serialize(serializer),
            OneOfDurationOrIso8601Expression::Iso8601Expression(s) => serializer.serialize_str(s),
        }
    }
}

impl Default for OneOfDurationOrIso8601Expression {
    fn default() -> Self {
        // Choose a default variant
        OneOfDurationOrIso8601Expression::Duration(Duration::default())
    }
}
impl fmt::Display for OneOfDurationOrIso8601Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OneOfDurationOrIso8601Expression::Duration(duration) => write!(f, "{}", duration),
            OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => write!(f, "{}", expr),
        }
    }
}

impl From<Duration> for OneOfDurationOrIso8601Expression {
    fn from(duration: Duration) -> Self {
        OneOfDurationOrIso8601Expression::Duration(duration)
    }
}

impl OneOfDurationOrIso8601Expression {
    /// Gets the total milliseconds for inline Duration variant.
    /// Returns 0 for ISO8601 expression variant (needs runtime parsing).
    pub fn total_milliseconds(&self) -> u64 {
        match self {
            OneOfDurationOrIso8601Expression::Duration(d) => d.total_milliseconds(),
            OneOfDurationOrIso8601Expression::Iso8601Expression(_) => 0,
        }
    }

    /// Returns true if this is an inline Duration variant
    pub fn is_duration(&self) -> bool {
        matches!(self, OneOfDurationOrIso8601Expression::Duration(_))
    }

    /// Returns true if this is an ISO8601 expression variant
    pub fn is_iso8601(&self) -> bool {
        matches!(self, OneOfDurationOrIso8601Expression::Iso8601Expression(_))
    }

    /// Gets the ISO8601 expression string, if this is an expression variant
    pub fn as_iso8601(&self) -> Option<&str> {
        match self {
            OneOfDurationOrIso8601Expression::Iso8601Expression(s) => Some(s),
            _ => None,
        }
    }
}

/// Validates whether an ISO 8601 duration string is supported by the Serverless Workflow spec.
/// Rejects: years (Y), weeks (W), months (M in date part), fractional days, bare P/PT,
/// and non-ISO formats.
/// Matches Go SDK's isISO8601DurationValid regex validator behavior.
pub fn is_iso8601_duration_valid(s: &str) -> bool {
    if !s.starts_with('P') {
        return false;
    }
    let rest = &s[1..];
    if rest.is_empty() {
        return false; // bare "P" is invalid
    }

    let (date_part, time_part) = if let Some(t_idx) = rest.find('T') {
        let date = &rest[..t_idx];
        let time = &rest[t_idx + 1..];
        if time.is_empty() {
            return false; // bare "PT" is invalid
        }
        (date, Some(time))
    } else {
        (rest, None)
    };

    // Date part: only integer days supported (no Y, W, M for months, fractional days)
    if !date_part.is_empty() {
        let days_str = date_part.trim_end_matches('D');
        if days_str.is_empty() && date_part.ends_with('D') {
            return false; // bare D
        }
        if !days_str.is_empty() {
            // Must be integer (no fractional days like P1.5D)
            if days_str.parse::<u64>().is_err() {
                return false;
            }
        }
        // Reject any Y, W, or M in date part
        if date_part.contains('Y') || date_part.contains('W') || date_part.contains('M') {
            return false;
        }
    }

    // Time part validation: parse component by component and ensure full consumption
    if let Some(time) = time_part {
        let remaining = parse_time_components(time);
        if remaining.is_none() {
            return false;
        }
        // If there's unparsed trailing content, it's invalid (e.g., "5MS7" leaves "7")
        if let Some(remaining) = remaining {
            if !remaining.is_empty() {
                return false;
            }
        }
    }

    true
}

/// Parses time components (H, M, S, MS) and returns the remaining unparsed string.
/// Returns None if the format is invalid.
fn parse_time_components(mut s: &str) -> Option<&str> {
    // Order matters: try MS (milliseconds) before M (minutes) and S (seconds)
    // We need to handle the custom "MS" suffix used by the Serverless Workflow spec
    while !s.is_empty() {
        // Try to parse a number followed by a unit
        let (num_str, rest) = split_number_prefix(s)?;
        if num_str.is_empty() {
            return None; // no number found
        }
        // Validate the number (allow integer or decimal for seconds)
        if num_str.parse::<f64>().is_err() {
            return None;
        }
        // Check for unit suffix: MS (milliseconds) must be checked before M and S
        if let Some(rest_after_ms) = rest.strip_prefix("MS") {
            s = rest_after_ms;
        } else if let Some(rest_after_h) = rest.strip_prefix('H') {
            s = rest_after_h;
        } else if let Some(rest_after_m) = rest.strip_prefix('M') {
            s = rest_after_m;
        } else if let Some(rest_after_s) = rest.strip_prefix('S') {
            s = rest_after_s;
        } else {
            // No recognized unit suffix — return remaining for caller to check
            return Some(s);
        }
    }
    Some(s) // empty string = fully consumed
}

/// Splits a string into the leading numeric portion and the rest.
fn split_number_prefix(s: &str) -> Option<(&str, &str)> {
    let mut i = 0;
    let bytes = s.as_bytes();
    // Allow optional leading minus (though durations shouldn't have it)
    if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }
    // Integer part
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Optional decimal part
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i == 0 {
        return None;
    }
    Some((&s[..i], &s[i..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_from_days() {
        let d = Duration::from_days(1);
        assert_eq!(d.days, Some(1));
        assert_eq!(d.total_milliseconds(), 86400000);
    }

    #[test]
    fn test_duration_from_hours() {
        let d = Duration::from_hours(2);
        assert_eq!(d.hours, Some(2));
        assert_eq!(d.total_milliseconds(), 7200000);
    }

    #[test]
    fn test_duration_from_minutes() {
        let d = Duration::from_minutes(5);
        assert_eq!(d.minutes, Some(5));
        assert_eq!(d.total_milliseconds(), 300000);
    }

    #[test]
    fn test_duration_from_seconds() {
        let d = Duration::from_seconds(30);
        assert_eq!(d.seconds, Some(30));
        assert_eq!(d.total_milliseconds(), 30000);
    }

    #[test]
    fn test_duration_from_milliseconds() {
        let d = Duration::from_milliseconds(500);
        assert_eq!(d.milliseconds, Some(500));
        assert_eq!(d.total_milliseconds(), 500);
    }

    #[test]
    fn test_duration_composite() {
        let d = Duration {
            days: Some(1),
            hours: Some(2),
            minutes: Some(30),
            seconds: Some(45),
            milliseconds: Some(500),
        };
        let expected = 86400000 + 7200000 + 1800000 + 45000 + 500;
        assert_eq!(d.total_milliseconds(), expected);
    }

    #[test]
    fn test_duration_total_conversions() {
        let d = Duration::from_minutes(90);
        assert_eq!(d.total_hours(), 1.5);
        assert_eq!(d.total_minutes(), 90.0);
        assert_eq!(d.total_seconds(), 5400.0);
    }

    #[test]
    fn test_duration_serialize() {
        let d = Duration::from_seconds(30);
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#"{"seconds":30}"#);
    }

    #[test]
    fn test_duration_deserialize() {
        let json = r#"{"minutes": 5, "seconds": 30}"#;
        let d: Duration = serde_json::from_str(json).unwrap();
        assert_eq!(d.minutes, Some(5));
        assert_eq!(d.seconds, Some(30));
    }

    #[test]
    fn test_duration_empty_object_rejected() {
        // Matches Go SDK: Duration.UnmarshalJSON rejects empty objects
        let json = r#"{}"#;
        let result: Result<Duration, _> = serde_json::from_str(json);
        assert!(result.is_err(), "empty duration object should be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("at least one property"),
            "expected 'at least one property' error, got: {}",
            err
        );
    }

    #[test]
    fn test_duration_unknown_key_rejected() {
        // Matches Go SDK: Duration.UnmarshalJSON rejects unknown keys
        let json = r#"{"after": "PT1S"}"#;
        let result: Result<Duration, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unknown key in duration object should be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unexpected key"),
            "expected 'unexpected key' error, got: {}",
            err
        );
    }

    #[test]
    fn test_duration_unknown_key_mixed_rejected() {
        // Unknown key mixed with valid keys should still be rejected
        let json = r#"{"seconds": 30, "duration": "PT1S"}"#;
        let result: Result<Duration, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unknown key mixed with valid keys should be rejected"
        );
    }

    #[test]
    fn test_duration_default() {
        let d = Duration::default();
        assert_eq!(d.total_milliseconds(), 0);
    }

    #[test]
    fn test_oneof_duration_serialize_struct() {
        let oneof = OneOfDurationOrIso8601Expression::Duration(Duration::from_seconds(30));
        let json = serde_json::to_string(&oneof).unwrap();
        assert_eq!(json, r#"{"seconds":30}"#);
    }

    #[test]
    fn test_oneof_duration_serialize_iso8601() {
        let oneof = OneOfDurationOrIso8601Expression::Iso8601Expression("PT5M".to_string());
        let json = serde_json::to_string(&oneof).unwrap();
        assert_eq!(json, r#""PT5M""#);
    }

    #[test]
    fn test_oneof_duration_deserialize_struct() {
        let json = r#"{"seconds": 30}"#;
        let oneof: OneOfDurationOrIso8601Expression = serde_json::from_str(json).unwrap();
        match oneof {
            OneOfDurationOrIso8601Expression::Duration(d) => {
                assert_eq!(d.seconds, Some(30));
            }
            _ => panic!("Expected Duration variant"),
        }
    }

    #[test]
    fn test_oneof_duration_deserialize_iso8601() {
        let json = r#""PT5M""#;
        let oneof: OneOfDurationOrIso8601Expression = serde_json::from_str(json).unwrap();
        match oneof {
            OneOfDurationOrIso8601Expression::Iso8601Expression(s) => {
                assert_eq!(s, "PT5M");
            }
            _ => panic!("Expected Iso8601Expression variant"),
        }
    }

    #[test]
    fn test_duration_display() {
        let d = Duration {
            hours: Some(2),
            minutes: Some(30),
            ..Default::default()
        };
        let display = format!("{}", d);
        assert!(display.contains("2 hours"));
        assert!(display.contains("30 minutes"));
    }

    // Additional tests matching Go SDK's duration_test.go and validator_test.go

    #[test]
    fn test_oneof_iso8601_valid_patterns() {
        // Valid ISO8601 patterns accepted by Go SDK
        let valid_cases = vec![
            ("\"P1D\"", "P1D"),
            ("\"P1DT12H30M\"", "P1DT12H30M"),
            ("\"PT1H\"", "PT1H"),
            ("\"PT250MS\"", "PT250MS"),
            ("\"P3DT4H5M6S250MS\"", "P3DT4H5M6S250MS"),
        ];
        for (json, expected) in valid_cases {
            let oneof: OneOfDurationOrIso8601Expression = serde_json::from_str(json).unwrap();
            match &oneof {
                OneOfDurationOrIso8601Expression::Iso8601Expression(s) => {
                    assert_eq!(s, expected, "expected ISO expression {}", expected);
                }
                _ => panic!("Expected Iso8601Expression variant for {}", expected),
            }
        }
    }

    #[test]
    fn test_oneof_iso8601_rejected_patterns() {
        // Patterns rejected by Go SDK validator — now rejected at deserialization time
        // (matches Go SDK's Duration.UnmarshalJSON which validates at parse time)
        let rejected = vec![
            "\"P2Y\"",     // years not supported
            "\"P1Y2M3D\"", // months not supported in date part
            "\"P1W\"",     // weeks not supported
            "\"1Y\"",      // missing P prefix
        ];
        for json in rejected {
            // These should now fail at deserialization (matching Go SDK behavior)
            let result: Result<OneOfDurationOrIso8601Expression, _> = serde_json::from_str(json);
            assert!(result.is_err(), "expected {} to fail deserialization", json);
        }
    }

    #[test]
    fn test_duration_composite_with_all_fields() {
        let d = Duration {
            days: Some(3),
            hours: Some(4),
            minutes: Some(5),
            seconds: Some(6),
            milliseconds: Some(250),
        };
        let expected = 3 * 86400000 + 4 * 3600000 + 5 * 60000 + 6 * 1000 + 250;
        assert_eq!(d.total_milliseconds(), expected);
    }

    // ISO8601 validation tests matching Go SDK's validator_test.go

    #[test]
    fn test_iso8601_duration_valid_patterns() {
        assert!(is_iso8601_duration_valid("P1D"), "P1D should be valid");
        assert!(
            is_iso8601_duration_valid("P1DT12H30M"),
            "P1DT12H30M should be valid"
        );
        assert!(is_iso8601_duration_valid("PT1H"), "PT1H should be valid");
        assert!(
            is_iso8601_duration_valid("PT250MS"),
            "PT250MS should be valid"
        );
        assert!(
            is_iso8601_duration_valid("P3DT4H5M6S250MS"),
            "P3DT4H5M6S250MS should be valid"
        );
        assert!(is_iso8601_duration_valid("PT30S"), "PT30S should be valid");
        assert!(
            is_iso8601_duration_valid("PT0.1S"),
            "PT0.1S should be valid"
        );
        assert!(
            is_iso8601_duration_valid("P1DT2H30M"),
            "P1DT2H30M should be valid"
        );
    }

    #[test]
    fn test_iso8601_duration_invalid_patterns() {
        // Matches Go SDK's validator_test.go rejected patterns
        assert!(!is_iso8601_duration_valid("P2Y"), "years not supported");
        assert!(
            !is_iso8601_duration_valid("P1Y2M3D"),
            "months not supported in date part"
        );
        assert!(!is_iso8601_duration_valid("P1W"), "weeks not supported");
        assert!(
            !is_iso8601_duration_valid("P1Y2M3D4H"),
            "years+months not supported"
        );
        assert!(
            !is_iso8601_duration_valid("P1Y2M3D4H5M6S"),
            "years+months not supported"
        );
        assert!(!is_iso8601_duration_valid("P"), "bare P is invalid");
        assert!(!is_iso8601_duration_valid("P1DT"), "bare PT is invalid");
        assert!(!is_iso8601_duration_valid("1Y"), "missing P prefix");
        assert!(!is_iso8601_duration_valid(""), "empty string is invalid");
        assert!(
            !is_iso8601_duration_valid("P1.5D"),
            "fractional days not supported"
        );
        assert!(
            !is_iso8601_duration_valid("P1M"),
            "months (M in date part) not supported"
        );
        // Additional from Go SDK validator_test.go
        assert!(
            !is_iso8601_duration_valid("P1DT2H3M4S5MS7"),
            "trailing garbage after MS not valid"
        );
    }

    // Additional duration validation tests matching Go SDK's duration_test.go

    #[test]
    fn test_iso8601_non_iso_rejected_patterns() {
        // Matches Go SDK: DurationToTime rejects non-ISO formats like "10s", "150ms"
        assert!(
            !is_iso8601_duration_valid("10s"),
            "non-ISO '10s' should be rejected"
        );
        assert!(
            !is_iso8601_duration_valid("150ms"),
            "non-ISO '150ms' should be rejected"
        );
        assert!(
            !is_iso8601_duration_valid("1Y"),
            "non-ISO '1Y' should be rejected"
        );
        assert!(
            !is_iso8601_duration_valid("PT"),
            "bare 'PT' should be rejected"
        );
    }

    #[test]
    fn test_iso8601_p1dt1h_valid() {
        // Matches Go SDK: DurationToTime with "P1DT1H" → 25 hours
        assert!(
            is_iso8601_duration_valid("P1DT1H"),
            "P1DT1H should be valid"
        );
    }

    #[test]
    fn test_iso8601_pt1s250ms_valid() {
        // Matches Go SDK: DurationToTime with "PT1S250MS" → 1250ms
        assert!(
            is_iso8601_duration_valid("PT1S250MS"),
            "PT1S250MS should be valid"
        );
    }

    #[test]
    fn test_iso8601_rejected_year() {
        // Matches Go SDK: DurationToTime_YearExpressionRejected
        assert!(
            !is_iso8601_duration_valid("P1Y"),
            "P1Y should be rejected (years)"
        );
    }

    #[test]
    fn test_iso8601_rejected_week() {
        // Matches Go SDK: DurationToTime_WeekExpressionRejected
        assert!(
            !is_iso8601_duration_valid("P1W"),
            "P1W should be rejected (weeks)"
        );
    }

    #[test]
    fn test_iso8601_rejected_fractional_day() {
        // Matches Go SDK: DurationToTime_FractionalDayExpressionRejected
        assert!(
            !is_iso8601_duration_valid("P1.5D"),
            "P1.5D should be rejected (fractional days)"
        );
    }

    #[test]
    fn test_iso8601_rejected_month() {
        // Matches Go SDK: DurationToTime_UnsupportedMonthExpressionRejected
        assert!(
            !is_iso8601_duration_valid("P1M"),
            "P1M should be rejected (months)"
        );
    }

    #[test]
    fn test_iso8601_rejected_bare_pt() {
        // Matches Go SDK: DurationToTime_InvalidBarePTExpressionRejected
        assert!(
            !is_iso8601_duration_valid("PT"),
            "bare PT should be rejected"
        );
    }

    #[test]
    fn test_iso8601_rejected_invalid_expression() {
        // Matches Go SDK: DurationToTime_InvalidExpression
        assert!(
            !is_iso8601_duration_valid("1Y"),
            "1Y without P prefix should be rejected"
        );
    }

    #[test]
    fn test_oneof_duration_roundtrip_struct() {
        let duration = OneOfDurationOrIso8601Expression::Duration(Duration {
            days: Some(1),
            hours: Some(2),
            minutes: Some(30),
            ..Default::default()
        });
        let serialized = serde_json::to_string(&duration).unwrap();
        let deserialized: OneOfDurationOrIso8601Expression =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(duration, deserialized);
    }

    #[test]
    fn test_oneof_duration_roundtrip_iso8601() {
        let duration =
            OneOfDurationOrIso8601Expression::Iso8601Expression("P3DT4H5M6S250MS".to_string());
        let serialized = serde_json::to_string(&duration).unwrap();
        let deserialized: OneOfDurationOrIso8601Expression =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(duration, deserialized);
    }
}
