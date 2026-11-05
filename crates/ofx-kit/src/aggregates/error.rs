//! Errors used while constructing typed OFX aggregates.
//!
//! These errors are retained as a separate domain type so aggregate validation
//! can evolve independently from XML and header errors.

/// Error type for OFX aggregate parsing failures.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AggregateError {
    /// A required field was missing from the aggregate.
    #[error("missing required field '{field}' in <{aggregate}>")]
    MissingRequiredField {
        /// Aggregate containing the missing field.
        aggregate: String,
        /// Name of the missing field.
        field: &'static str,
    },
    /// Two mutually exclusive fields were both present.
    #[error("mutually exclusive fields '{field_a}' and '{field_b}' both present in <{aggregate}>")]
    MutuallyExclusiveFields {
        /// Aggregate containing both fields.
        aggregate: String,
        /// First conflicting field.
        field_a: &'static str,
        /// Second conflicting field.
        field_b: &'static str,
    },
    /// A field contained an invalid value.
    #[error("invalid value '{value}' for field '{field}' in <{aggregate}>: {reason}")]
    InvalidFieldValue {
        /// Aggregate containing the invalid field.
        aggregate: String,
        /// Name of the invalid field.
        field: &'static str,
        /// Original field value.
        value: String,
        /// Explanation of the validation failure.
        reason: String,
    },
}
