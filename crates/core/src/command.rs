//! Normalized command domain types.

use crate::{ActionRisk, DownloadId, Permission, TorrentSource};

/// A user intent normalized by a chat adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand {
    /// Show available commands.
    Help,
    /// Show high-level Routerbot status.
    ShowStatus,
    /// List current downloads.
    ListDownloads,
    /// Add a torrent from a normalized source.
    AddTorrent {
        /// The source to add.
        source: TorrentSource,
    },
    /// Stop an existing download.
    StopDownload {
        /// The download to stop.
        id: DownloadId,
    },
    /// Trigger media indexing.
    RescanMedia,
    /// Reboot the configured device.
    RebootDevice,
}

impl BotCommand {
    /// Returns the permission required to execute this command.
    #[must_use]
    pub const fn required_permission(&self) -> Permission {
        match self {
            Self::Help => Permission::ViewHelp,
            Self::ShowStatus => Permission::ViewStatus,
            Self::ListDownloads => Permission::ListDownloads,
            Self::AddTorrent { .. } => Permission::AddTorrent,
            Self::StopDownload { .. } => Permission::StopDownload,
            Self::RescanMedia => Permission::RescanMedia,
            Self::RebootDevice => Permission::RebootDevice,
        }
    }

    /// Returns the risk class of this command.
    #[must_use]
    pub const fn action_risk(&self) -> ActionRisk {
        match self {
            Self::Help | Self::ShowStatus | Self::ListDownloads => ActionRisk::ReadOnly,
            Self::AddTorrent { .. } | Self::StopDownload { .. } | Self::RescanMedia => {
                ActionRisk::Mutating
            }
            Self::RebootDevice => ActionRisk::Dangerous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_download_id() -> DownloadId {
        DownloadId::new("download-1").expect("download id should be valid")
    }

    fn sample_torrent_source() -> TorrentSource {
        TorrentSource::Magnet("magnet:?xt=urn:btih:example".to_owned())
    }

    #[test]
    fn commands_map_to_required_permissions() {
        let cases = [
            (BotCommand::Help, Permission::ViewHelp),
            (BotCommand::ShowStatus, Permission::ViewStatus),
            (BotCommand::ListDownloads, Permission::ListDownloads),
            (BotCommand::AddTorrent { source: sample_torrent_source() }, Permission::AddTorrent),
            (BotCommand::StopDownload { id: sample_download_id() }, Permission::StopDownload),
            (BotCommand::RescanMedia, Permission::RescanMedia),
            (BotCommand::RebootDevice, Permission::RebootDevice),
        ];

        for (command, permission) in cases {
            assert_eq!(command.required_permission(), permission);
        }
    }

    #[test]
    fn commands_map_to_action_risks() {
        let cases = [
            (BotCommand::Help, ActionRisk::ReadOnly),
            (BotCommand::ShowStatus, ActionRisk::ReadOnly),
            (BotCommand::ListDownloads, ActionRisk::ReadOnly),
            (BotCommand::AddTorrent { source: sample_torrent_source() }, ActionRisk::Mutating),
            (BotCommand::StopDownload { id: sample_download_id() }, ActionRisk::Mutating),
            (BotCommand::RescanMedia, ActionRisk::Mutating),
            (BotCommand::RebootDevice, ActionRisk::Dangerous),
        ];

        for (command, risk) in cases {
            assert_eq!(command.action_risk(), risk);
        }
    }
}
