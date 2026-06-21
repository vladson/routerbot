//! Authorization domain types.

/// A permission required to execute a normalized bot command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Permission to view command help.
    ViewHelp,
    /// Permission to view high-level Routerbot status.
    ViewStatus,
    /// Permission to list downloads.
    ListDownloads,
    /// Permission to add a torrent.
    AddTorrent,
    /// Permission to stop a download.
    StopDownload,
    /// Permission to trigger media indexing.
    RescanMedia,
    /// Permission to reboot the configured device.
    RebootDevice,
    /// Permission to restart a configured workload.
    RestartWorkload,
}

/// The risk class of a command or operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRisk {
    /// The operation only reads state.
    ReadOnly,
    /// The operation changes state but is not intrinsically dangerous.
    Mutating,
    /// The operation can disrupt service and must require confirmation.
    Dangerous,
}
