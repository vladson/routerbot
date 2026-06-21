//! Core domain crate for Routerbot.
//!
//! This crate contains adapter-neutral command and state types shared by the
//! application layer and concrete integrations.

mod auth;
mod command;
mod download;
mod error;
mod media;
mod workload;

pub use auth::{ActionRisk, Permission};
pub use command::BotCommand;
pub use download::{DownloadId, DownloadState, DownloadStatus, TorrentSource};
pub use error::DomainError;
pub use media::MediaRescanResult;
pub use workload::{WorkloadKind, WorkloadStatus, WorkloadTarget};
