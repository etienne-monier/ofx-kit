//! Errors used while constructing typed OFX aggregates.
//!
//! These errors are retained as a separate domain type so aggregate validation
//! can evolve independently from XML and header errors.

use core::fmt;

/// Error type for OFX aggregate parsing failures.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateError {
    /// A required field was missing from the aggregate.
    MissingRequiredField {
        /// Aggregate containing the missing field.
        aggregate: String,
        /// Name of the missing field.
        field: &'static str,
    },
    /// Two mutually exclusive fields were both present.
    MutuallyExclusiveFields {
        /// Aggregate containing both fields.
        aggregate: String,
        /// First conflicting field.
        field_a: &'static str,
        /// Second conflicting field.
        field_b: &'static str,
    },
    /// A field contained an invalid value.
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

impl fmt::Display for AggregateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField { aggregate, field } => {
                write!(f, "missing required field '{field}' in <{aggregate}>")
            }
            Self::MutuallyExclusiveFields {
                aggregate,
                field_a,
                field_b,
            } => {
                write!(
                    f,
                    "mutually exclusive fields '{field_a}' and '{field_b}' both present in <{aggregate}>"
                )
            }
            Self::InvalidFieldValue {
                aggregate,
                field,
                value,
                reason,
            } => {
                write!(
                    f,
                    "invalid value '{value}' for field '{field}' in <{aggregate}>: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for AggregateError {}
