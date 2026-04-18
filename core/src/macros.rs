/// Macro to define a newtype-style constant struct with string constants.
///
/// Generates a unit struct with `pub const` string fields and an `ALL_VALUES` array.
///
/// # Example
/// ```ignore
/// string_constants! {
///     /// Enumerates all supported task types
///     TaskType {
///         CALL => "call",
///         DO => "do",
///     }
/// }
/// ```
macro_rules! string_constants {
    ($(#[$meta:meta])* $name:ident { $( $(#[$cmeta:meta])* $field:ident => $val:literal ),* $(,)? }) => {
        $(#[$meta])*
        pub struct $name;
        impl $name {
            $(
                $(#[$cmeta])*
                pub const $field: &'static str = $val;
            )*
            /// All valid values for this type
            pub const ALL_VALUES: &'static [&'static str] = &[$($val),*];
        }
    };
}

/// Macro to define a `serde(untagged)` enum with a concrete definition variant
/// and a `Reference(String)` variant, plus a `Default` impl.
///
/// Supports both direct (`Inner`) and boxed (`Box<Inner>`) inner types.
///
/// # Example
/// ```ignore
/// define_one_of_or_reference!(
///     /// A timeout or reference
///     OneOfTimeoutDefinitionOrReference, Timeout(TimeoutDefinition)
/// );
/// define_one_of_or_reference!(
///     /// A retry policy or reference
///     OneOfRetryPolicyDefinitionOrReference, Retry(Box<RetryPolicyDefinition>)
/// );
/// ```
macro_rules! define_one_of_or_reference {
    ($(#[$meta:meta])* $name:ident, $variant:ident($inner:ty)) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(untagged)]
        pub enum $name {
            $variant($inner),
            /// Variant holding a reference
            Reference(String),
        }
        impl Default for $name {
            fn default() -> Self {
                $name::$variant(Default::default())
            }
        }
    };
}
