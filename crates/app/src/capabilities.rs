//! Application capability traits.

use std::{future::Future, pin::Pin, time::Duration};

use routerbot_core::{
    DownloadId, DownloadStatus, MediaRescanResult, TorrentSource, WorkloadStatus, WorkloadTarget,
};

use crate::AppError;

/// A boxed future returned by object-safe capability traits.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Download management capability.
pub trait DownloadControl: Send + Sync {
    /// Adds a torrent and returns its download identifier.
    fn add(&self, source: TorrentSource) -> BoxFuture<'_, Result<DownloadId, AppError>>;

    /// Lists current downloads.
    fn list(&self) -> BoxFuture<'_, Result<Vec<DownloadStatus>, AppError>>;

    /// Stops a download.
    fn stop(&self, id: DownloadId) -> BoxFuture<'_, Result<(), AppError>>;
}

/// Device management capability.
pub trait DeviceControl: Send + Sync {
    /// Reboots the configured device.
    fn reboot(&self) -> BoxFuture<'_, Result<(), AppError>>;

    /// Returns device uptime when available.
    fn uptime(&self) -> BoxFuture<'_, Result<Option<Duration>, AppError>>;
}

/// Media indexing capability.
pub trait MediaIndexControl: Send + Sync {
    /// Triggers media indexing.
    fn rescan(&self) -> BoxFuture<'_, Result<MediaRescanResult, AppError>>;
}

/// Workload management capability.
pub trait WorkloadControl: Send + Sync {
    /// Restarts the target workload.
    fn restart_workload(&self, target: WorkloadTarget) -> BoxFuture<'_, Result<(), AppError>>;

    /// Returns target workload status.
    fn workload_status(
        &self,
        target: WorkloadTarget,
    ) -> BoxFuture<'_, Result<WorkloadStatus, AppError>>;
}
