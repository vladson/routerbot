//! Capability registry.

use std::sync::Arc;

use crate::{
    AppError, Capability, DeviceControl, DownloadControl, MediaIndexControl, WorkloadControl,
};

/// Registry of configured application capabilities.
#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    downloads: Option<Arc<dyn DownloadControl>>,
    device: Option<Arc<dyn DeviceControl>>,
    media_index: Option<Arc<dyn MediaIndexControl>>,
    workload: Option<Arc<dyn WorkloadControl>>,
}

impl CapabilityRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers download management capability.
    #[must_use]
    pub fn with_downloads(mut self, downloads: impl DownloadControl + 'static) -> Self {
        self.downloads = Some(Arc::new(downloads));
        self
    }

    /// Registers device management capability.
    #[must_use]
    pub fn with_device(mut self, device: impl DeviceControl + 'static) -> Self {
        self.device = Some(Arc::new(device));
        self
    }

    /// Registers media indexing capability.
    #[must_use]
    pub fn with_media_index(mut self, media_index: impl MediaIndexControl + 'static) -> Self {
        self.media_index = Some(Arc::new(media_index));
        self
    }

    /// Registers workload management capability.
    #[must_use]
    pub fn with_workload(mut self, workload: impl WorkloadControl + 'static) -> Self {
        self.workload = Some(Arc::new(workload));
        self
    }

    /// Returns the download management capability.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::MissingCapability`] when downloads are not configured.
    pub fn downloads(&self) -> Result<Arc<dyn DownloadControl>, AppError> {
        self.downloads
            .clone()
            .ok_or(AppError::MissingCapability { capability: Capability::Downloads })
    }

    /// Returns the device management capability.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::MissingCapability`] when device control is not configured.
    pub fn device(&self) -> Result<Arc<dyn DeviceControl>, AppError> {
        self.device.clone().ok_or(AppError::MissingCapability { capability: Capability::Device })
    }

    /// Returns the media indexing capability.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::MissingCapability`] when media indexing is not configured.
    pub fn media_index(&self) -> Result<Arc<dyn MediaIndexControl>, AppError> {
        self.media_index
            .clone()
            .ok_or(AppError::MissingCapability { capability: Capability::MediaIndex })
    }

    /// Returns the workload management capability.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::MissingCapability`] when workload control is not configured.
    pub fn workload(&self) -> Result<Arc<dyn WorkloadControl>, AppError> {
        self.workload
            .clone()
            .ok_or(AppError::MissingCapability { capability: Capability::Workload })
    }
}
