//! Application orchestration crate for Routerbot.
//!
//! This crate owns authorization, capability routing, and command execution.

mod authz;
mod capabilities;
mod error;
mod registry;
mod service;

pub use authz::AuthzService;
pub use capabilities::{
    BoxFuture, DeviceControl, DownloadControl, MediaIndexControl, WorkloadControl,
};
pub use error::{AppError, Capability};
pub use registry::CapabilityRegistry;
pub use service::{CommandResponse, CommandService};
