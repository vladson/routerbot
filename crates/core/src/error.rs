//! Domain-level errors.

/// An error produced while constructing or validating domain values.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A required string field was empty or whitespace-only.
    #[error("{field} must not be empty")]
    EmptyValue {
        /// The invalid field name.
        field: &'static str,
    },
    /// A progress percentage was outside the accepted 0..=100 range.
    #[error("progress percent must be between 0 and 100, got {value}")]
    InvalidProgressPercent {
        /// The invalid progress value.
        value: u8,
    },
}
