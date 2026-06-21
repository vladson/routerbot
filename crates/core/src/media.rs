//! Media indexing domain types.

/// The result of a media indexing request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRescanResult {
    /// Whether a rescan operation was triggered.
    pub triggered: bool,
    /// Optional human-readable result message.
    pub message: Option<String>,
}

impl MediaRescanResult {
    /// Creates a result for a triggered media rescan.
    #[must_use]
    pub fn triggered(message: impl Into<String>) -> Self {
        Self { triggered: true, message: Some(message.into()) }
    }

    /// Creates a result for a skipped media rescan.
    #[must_use]
    pub fn skipped(message: impl Into<String>) -> Self {
        Self { triggered: false, message: Some(message.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggered_result_preserves_state() {
        let result = MediaRescanResult::triggered("rescan started");

        assert!(result.triggered);
        assert_eq!(result.message.as_deref(), Some("rescan started"));
    }

    #[test]
    fn skipped_result_preserves_state() {
        let result = MediaRescanResult::skipped("adapter disabled");

        assert!(!result.triggered);
        assert_eq!(result.message.as_deref(), Some("adapter disabled"));
    }
}
