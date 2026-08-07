//! User-facing Telegram message formatting.

use routerbot_app::{AppError, CommandResponse};
use routerbot_core::{BotCommand, DownloadState, DownloadStatus};

const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

pub(crate) const HELP_TEXT: &str = "Routerbot commands:\n\
/status — show Routerbot status\n\
/downloads — list downloads\n\
/add <magnet-or-http(s)-url> — add a torrent\n\
/stop <download-id> — stop a download\n\
/remove <download-id> [--delete-data] — remove a download\n\
/rescan — rescan media\n\
/reboot — reboot the configured device\n\
/help — show this help";

/// Formats an application command response as Telegram-safe plain text.
///
/// The returned text does not use a Telegram parse mode and is limited to the
/// Telegram message length boundary.
#[must_use]
pub fn format_command_response(response: &CommandResponse) -> String {
    let text = match response {
        CommandResponse::Help => HELP_TEXT.to_owned(),
        CommandResponse::Status { device_uptime } => device_uptime.map_or_else(
            || "Routerbot is running.".to_owned(),
            |uptime| format!("Routerbot is running.\nDevice uptime: {}.", format_duration(uptime)),
        ),
        CommandResponse::Downloads(downloads) => format_downloads(downloads),
        CommandResponse::TorrentAdded(id) => format!("Torrent added: {}", id.as_str()),
        CommandResponse::DownloadStopped(id) => format!("Download stopped: {}", id.as_str()),
        CommandResponse::DownloadRemoved { id, data_deleted } => {
            if *data_deleted {
                format!("Download and local data removed: {}", id.as_str())
            } else {
                format!("Download removed; local data preserved: {}", id.as_str())
            }
        }
        CommandResponse::MediaRescan(result) => {
            let status =
                if result.triggered { "Media rescan started." } else { "Media rescan skipped." };
            result
                .message
                .as_ref()
                .map_or_else(|| status.to_owned(), |message| format!("{status} {message}"))
        }
        CommandResponse::DeviceRebootStarted => "Device reboot started.".to_owned(),
    };
    truncate_utf16(text, TELEGRAM_MESSAGE_LIMIT)
}

fn format_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    let minutes = seconds % 3_600 / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

pub(crate) fn format_app_error(error: &AppError) -> &'static str {
    match error {
        AppError::Unauthorized { .. } => "Access denied.",
        AppError::MissingCapability { .. } => "This command is not available.",
        AppError::CapabilityFailed { .. } => "Command failed. Check Routerbot logs.",
    }
}

pub(crate) fn format_confirmation(command: &BotCommand) -> String {
    match command {
        BotCommand::RemoveDownload { id, delete_data: true } => {
            format!("Delete download {} and all local data? This cannot be undone.", id.as_str())
        }
        BotCommand::RebootDevice => {
            "Reboot the configured device? Routerbot may be unavailable temporarily.".to_owned()
        }
        _ => "Confirm this dangerous action?".to_owned(),
    }
}

fn format_downloads(downloads: &[DownloadStatus]) -> String {
    if downloads.is_empty() {
        return "No downloads.".to_owned();
    }

    let mut output = String::from("Downloads:");
    for download in downloads {
        let line = format!("\n{}", format_download(download));
        if utf16_len(&output) + utf16_len(&line) > TELEGRAM_MESSAGE_LIMIT - 24 {
            output.push_str("\nMore downloads omitted.");
            break;
        }
        output.push_str(&line);
    }
    truncate_utf16(output, TELEGRAM_MESSAGE_LIMIT)
}

fn format_download(download: &DownloadStatus) -> String {
    let mut line = format!(
        "{} — {} — {} — {}%",
        download.id().as_str(),
        download.name(),
        format_state(download.state()),
        download.progress_percent()
    );
    if let Some(rate) = download.download_rate_bytes_per_second() {
        line.push_str(" — down ");
        line.push_str(&format_bytes(rate));
        line.push_str("/s");
    }
    if let Some(rate) = download.upload_rate_bytes_per_second() {
        line.push_str(" — up ");
        line.push_str(&format_bytes(rate));
        line.push_str("/s");
    }
    line
}

const fn format_state(state: DownloadState) -> &'static str {
    match state {
        DownloadState::Queued => "queued",
        DownloadState::Downloading => "downloading",
        DownloadState::Paused => "paused",
        DownloadState::Completed => "completed",
        DownloadState::Seeding => "seeding",
        DownloadState::Stopped => "stopped",
        DownloadState::Error => "error",
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format_unit(bytes, GIB, "GiB")
    } else if bytes >= MIB {
        format_unit(bytes, MIB, "MiB")
    } else if bytes >= KIB {
        format_unit(bytes, KIB, "KiB")
    } else {
        format!("{bytes} B")
    }
}

fn format_unit(bytes: u64, unit: u64, suffix: &str) -> String {
    let whole = bytes / unit;
    let tenths = (bytes % unit) * 10 / unit;
    format!("{whole}.{tenths} {suffix}")
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn truncate_utf16(mut value: String, limit: usize) -> String {
    if utf16_len(&value) <= limit {
        return value;
    }
    let mut units = 0;
    let byte_index = value
        .char_indices()
        .find_map(|(index, character)| {
            units += character.len_utf16();
            (units > limit - 1).then_some(index)
        })
        .unwrap_or(value.len());
    value.truncate(byte_index);
    value.push('…');
    value
}

#[cfg(test)]
mod tests {
    use routerbot_core::{DownloadId, MediaRescanResult};

    use super::*;

    fn download(name: impl Into<String>) -> DownloadStatus {
        DownloadStatus::new(
            DownloadId::new("abc").unwrap(),
            name,
            DownloadState::Downloading,
            50,
            Some(1536),
            Some(0),
        )
        .unwrap()
    }

    #[test]
    fn formats_download_details() {
        let text =
            format_command_response(&CommandResponse::Downloads(vec![download("Linux ISO")]));

        assert_eq!(
            text,
            "Downloads:\nabc — Linux ISO — downloading — 50% — down 1.5 KiB/s — up 0 B/s"
        );
    }

    #[test]
    fn limits_long_unicode_download_output() {
        let text =
            format_command_response(&CommandResponse::Downloads(vec![download("😀".repeat(5000))]));

        assert!(utf16_len(&text) <= TELEGRAM_MESSAGE_LIMIT);
    }

    #[test]
    fn formats_empty_downloads_and_media_results() {
        assert_eq!(format_command_response(&CommandResponse::Downloads(vec![])), "No downloads.");
        assert_eq!(
            format_command_response(&CommandResponse::MediaRescan(MediaRescanResult::skipped(
                "busy"
            ))),
            "Media rescan skipped. busy"
        );
    }

    #[test]
    fn formats_device_uptime() {
        let response =
            CommandResponse::Status { device_uptime: Some(std::time::Duration::from_secs(62_055)) };
        assert_eq!(
            format_command_response(&response),
            "Routerbot is running.\nDevice uptime: 17h 14m."
        );
    }

    #[test]
    fn limits_long_external_messages() {
        let response =
            CommandResponse::MediaRescan(MediaRescanResult::triggered("😀".repeat(5000)));

        assert!(utf16_len(&format_command_response(&response)) <= TELEGRAM_MESSAGE_LIMIT);
    }
}
