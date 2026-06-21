//! Download domain types.

use crate::DomainError;

/// A torrent source normalized before adapter execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorrentSource {
    /// A magnet URI.
    Magnet(String),
    /// A URL pointing to torrent metadata or a downloadable source.
    Url(String),
    /// The name of a received torrent file.
    TorrentFileName(String),
}

/// A stable download identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DownloadId(String);

impl DownloadId {
    /// Creates a download identifier.
    ///
    /// Empty and whitespace-only identifiers are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::EmptyValue`] when the identifier is empty or
    /// whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::EmptyValue { field: "download_id" });
        }

        Ok(Self(value))
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The normalized state of a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    /// The download is queued.
    Queued,
    /// The download is currently downloading.
    Downloading,
    /// The download is paused.
    Paused,
    /// The download has completed.
    Completed,
    /// The download is seeding.
    Seeding,
    /// The download has stopped.
    Stopped,
    /// The download is in an error state.
    Error,
}

/// A normalized download status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadStatus {
    /// Stable download identifier.
    pub id: DownloadId,
    /// Human-readable download name.
    pub name: String,
    /// Current normalized download state.
    pub state: DownloadState,
    /// Completion progress from 0 to 100.
    pub progress_percent: u8,
    /// Optional download rate in bytes per second.
    pub download_rate_bytes_per_second: Option<u64>,
    /// Optional upload rate in bytes per second.
    pub upload_rate_bytes_per_second: Option<u64>,
}

impl DownloadStatus {
    /// Creates a normalized download status.
    ///
    /// Empty names and progress values above 100 are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::EmptyValue`] when the name is empty or
    /// whitespace-only. Returns [`DomainError::InvalidProgressPercent`] when
    /// `progress_percent` is greater than 100.
    pub fn new(
        id: DownloadId,
        name: impl Into<String>,
        state: DownloadState,
        progress_percent: u8,
        download_rate_bytes_per_second: Option<u64>,
        upload_rate_bytes_per_second: Option<u64>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyValue { field: "download_name" });
        }
        if progress_percent > 100 {
            return Err(DomainError::InvalidProgressPercent { value: progress_percent });
        }

        Ok(Self {
            id,
            name,
            state,
            progress_percent,
            download_rate_bytes_per_second,
            upload_rate_bytes_per_second,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_id_accepts_non_empty_values() {
        let id = DownloadId::new("abc-123").expect("download id should be valid");

        assert_eq!(id.as_str(), "abc-123");
    }

    #[test]
    fn download_id_rejects_empty_values() {
        assert_eq!(
            DownloadId::new("   ").expect_err("download id should be invalid"),
            DomainError::EmptyValue { field: "download_id" }
        );
    }

    #[test]
    fn download_status_accepts_progress_boundaries() {
        let id = DownloadId::new("1").expect("download id should be valid");

        let zero =
            DownloadStatus::new(id.clone(), "Ubuntu ISO", DownloadState::Queued, 0, None, None)
                .expect("zero progress should be valid");
        let complete = DownloadStatus::new(
            id,
            "Ubuntu ISO",
            DownloadState::Completed,
            100,
            Some(0),
            Some(1024),
        )
        .expect("complete progress should be valid");

        assert_eq!(zero.progress_percent, 0);
        assert_eq!(complete.progress_percent, 100);
    }

    #[test]
    fn download_status_rejects_progress_above_one_hundred() {
        let id = DownloadId::new("1").expect("download id should be valid");

        assert_eq!(
            DownloadStatus::new(id, "Ubuntu ISO", DownloadState::Downloading, 101, None, None)
                .expect_err("progress should be invalid"),
            DomainError::InvalidProgressPercent { value: 101 }
        );
    }

    #[test]
    fn download_status_rejects_empty_names() {
        let id = DownloadId::new("1").expect("download id should be valid");

        assert_eq!(
            DownloadStatus::new(id, "  ", DownloadState::Downloading, 50, None, None)
                .expect_err("download name should be invalid"),
            DomainError::EmptyValue { field: "download_name" }
        );
    }
}
