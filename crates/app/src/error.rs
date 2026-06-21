//! Application-level errors.

use routerbot_core::Permission;

/// A named application capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Download management capability.
    Downloads,
    /// Device management capability.
    Device,
    /// Media indexing capability.
    MediaIndex,
    /// Workload management capability.
    Workload,
}

/// An application-layer error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// The command requires a permission that was not granted.
    #[error("permission {permission:?} is not allowed")]
    Unauthorized {
        /// Permission required by the command.
        permission: Permission,
    },
    /// A required capability is not configured.
    #[error("capability {capability:?} is not configured")]
    MissingCapability {
        /// Missing capability.
        capability: Capability,
    },
    /// A configured capability failed.
    #[error("capability {capability:?} failed: {message}")]
    CapabilityFailed {
        /// Capability that failed.
        capability: Capability,
        /// Human-readable failure message.
        message: String,
    },
}
