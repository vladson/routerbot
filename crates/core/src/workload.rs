//! Workload domain types.

use crate::DomainError;

/// The kind of workload targeted by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadKind {
    /// A Kubernetes-style Deployment.
    Deployment,
}

/// A configured workload target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadTarget {
    /// Workload kind.
    pub kind: WorkloadKind,
    /// Optional namespace containing the workload.
    pub namespace: Option<String>,
    /// Workload name.
    pub name: String,
}

impl WorkloadTarget {
    /// Creates a workload target.
    ///
    /// Empty and whitespace-only names are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::EmptyValue`] when the workload name is empty or
    /// whitespace-only.
    pub fn new(
        kind: WorkloadKind,
        namespace: Option<String>,
        name: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyValue { field: "workload_name" });
        }

        Ok(Self { kind, namespace, name })
    }
}

/// Normalized workload status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadStatus {
    /// Workload target.
    pub target: WorkloadTarget,
    /// Desired replica count, when known.
    pub desired_replicas: Option<u32>,
    /// Ready replica count, when known.
    pub ready_replicas: Option<u32>,
    /// Whether the workload is available.
    pub available: bool,
    /// Optional human-readable status message.
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_target_accepts_non_empty_name() {
        let target =
            WorkloadTarget::new(WorkloadKind::Deployment, Some("media".to_owned()), "minidlna")
                .expect("workload target should be valid");

        assert_eq!(target.kind, WorkloadKind::Deployment);
        assert_eq!(target.namespace.as_deref(), Some("media"));
        assert_eq!(target.name, "minidlna");
    }

    #[test]
    fn workload_target_rejects_empty_name() {
        assert_eq!(
            WorkloadTarget::new(WorkloadKind::Deployment, None, " ")
                .expect_err("workload target should be invalid"),
            DomainError::EmptyValue { field: "workload_name" }
        );
    }
}
