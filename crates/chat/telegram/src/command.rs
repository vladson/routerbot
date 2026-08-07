//! Telegram command parsing and domain normalization.

use std::{error::Error, fmt};

use routerbot_core::{BotCommand, DownloadId, TorrentSource};
use url::Url;

const HELP_USAGE: &str = "Usage: /help";
const STATUS_USAGE: &str = "Usage: /status";
const DOWNLOADS_USAGE: &str = "Usage: /downloads";
const ADD_USAGE: &str = "Usage: /add <magnet-or-http(s)-url>";
const STOP_USAGE: &str = "Usage: /stop <download-id>";
const REMOVE_USAGE: &str = "Usage: /remove <download-id> [--delete-data]";
const RESCAN_USAGE: &str = "Usage: /rescan";
const REBOOT_USAGE: &str = "Usage: /reboot";

/// A parsed Telegram command before conversion to the core command model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramCommand {
    /// Show command help.
    Help,
    /// Show Routerbot status.
    Status,
    /// List downloads.
    Downloads,
    /// Add a torrent from a magnet or URL.
    Add(String),
    /// Stop a download.
    Stop(String),
    /// Remove a download.
    Remove {
        /// Stable download identifier.
        id: String,
        /// Whether downloaded data must also be deleted.
        delete_data: bool,
    },
    /// Trigger media indexing.
    Rescan,
    /// Reboot the configured device.
    Reboot,
}

impl TelegramCommand {
    /// Parses Telegram command text.
    ///
    /// Returns `Ok(None)` for ordinary text and commands addressed to another
    /// bot. Bot usernames are compared case-insensitively.
    ///
    /// # Errors
    ///
    /// Returns [`TelegramCommandParseError`] for unknown commands or invalid
    /// arguments.
    pub fn parse(
        text: &str,
        bot_username: Option<&str>,
    ) -> Result<Option<Self>, TelegramCommandParseError> {
        let text = text.trim();
        if !text.starts_with('/') {
            return Ok(None);
        }

        let (head, arguments) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        let (name, mention) = head[1..].split_once('@').unwrap_or((&head[1..], ""));
        if !mention.is_empty()
            && bot_username.is_none_or(|username| !mention.eq_ignore_ascii_case(username))
        {
            return Ok(None);
        }

        let arguments = arguments.trim();
        match name.to_ascii_lowercase().as_str() {
            "start" | "help" => no_arguments(arguments, Self::Help, HELP_USAGE),
            "status" => no_arguments(arguments, Self::Status, STATUS_USAGE),
            "downloads" => no_arguments(arguments, Self::Downloads, DOWNLOADS_USAGE),
            "add" => one_argument(arguments, ADD_USAGE).map(|value| Some(Self::Add(value))),
            "stop" => one_argument(arguments, STOP_USAGE).map(|value| Some(Self::Stop(value))),
            "remove" => parse_remove(arguments).map(Some),
            "rescan" => no_arguments(arguments, Self::Rescan, RESCAN_USAGE),
            "reboot" => no_arguments(arguments, Self::Reboot, REBOOT_USAGE),
            _ => Err(TelegramCommandParseError::new(
                "Unknown command. Use /help to list available commands.",
            )),
        }
    }

    /// Converts this Telegram command to the normalized core command model.
    ///
    /// # Errors
    ///
    /// Returns [`TelegramCommandParseError`] when a source URL or download ID
    /// is invalid.
    pub fn into_bot_command(self) -> Result<BotCommand, TelegramCommandParseError> {
        match self {
            Self::Help => Ok(BotCommand::Help),
            Self::Status => Ok(BotCommand::ShowStatus),
            Self::Downloads => Ok(BotCommand::ListDownloads),
            Self::Add(value) => Ok(BotCommand::AddTorrent { source: parse_source(value)? }),
            Self::Stop(id) => Ok(BotCommand::StopDownload { id: parse_download_id(id)? }),
            Self::Remove { id, delete_data } => {
                Ok(BotCommand::RemoveDownload { id: parse_download_id(id)?, delete_data })
            }
            Self::Rescan => Ok(BotCommand::RescanMedia),
            Self::Reboot => Ok(BotCommand::RebootDevice),
        }
    }
}

fn no_arguments(
    arguments: &str,
    command: TelegramCommand,
    usage: &'static str,
) -> Result<Option<TelegramCommand>, TelegramCommandParseError> {
    if arguments.is_empty() {
        Ok(Some(command))
    } else {
        Err(TelegramCommandParseError::new(usage))
    }
}

fn one_argument(arguments: &str, usage: &'static str) -> Result<String, TelegramCommandParseError> {
    if arguments.is_empty() || arguments.split_whitespace().count() != 1 {
        Err(TelegramCommandParseError::new(usage))
    } else {
        Ok(arguments.to_owned())
    }
}

fn parse_remove(arguments: &str) -> Result<TelegramCommand, TelegramCommandParseError> {
    let parts = arguments.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [id] => Ok(TelegramCommand::Remove { id: (*id).to_owned(), delete_data: false }),
        [id, "--delete-data"] => {
            Ok(TelegramCommand::Remove { id: (*id).to_owned(), delete_data: true })
        }
        _ => Err(TelegramCommandParseError::new(REMOVE_USAGE)),
    }
}

fn parse_source(value: String) -> Result<TorrentSource, TelegramCommandParseError> {
    let url = Url::parse(&value)
        .map_err(|_| TelegramCommandParseError::new("The torrent source is not a valid URL."))?;
    match url.scheme() {
        "magnet" => Ok(TorrentSource::Magnet(value)),
        "http" | "https" => Ok(TorrentSource::Url(value)),
        _ => Err(TelegramCommandParseError::new(
            "Only magnet, HTTP, and HTTPS torrent sources are supported.",
        )),
    }
}

fn parse_download_id(value: String) -> Result<DownloadId, TelegramCommandParseError> {
    DownloadId::new(value)
        .map_err(|_| TelegramCommandParseError::new("The download ID must not be empty."))
}

/// Error returned when Telegram command text is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramCommandParseError {
    message: &'static str,
}

impl TelegramCommandParseError {
    const fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl fmt::Display for TelegramCommandParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for TelegramCommandParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_commands() {
        let cases = [
            ("/start", TelegramCommand::Help),
            ("/help@RouterBot", TelegramCommand::Help),
            ("/status", TelegramCommand::Status),
            ("/downloads", TelegramCommand::Downloads),
            (
                "/add magnet:?xt=urn:btih:test",
                TelegramCommand::Add("magnet:?xt=urn:btih:test".into()),
            ),
            ("/stop abc", TelegramCommand::Stop("abc".into())),
            ("/remove abc", TelegramCommand::Remove { id: "abc".into(), delete_data: false }),
            (
                "/remove abc --delete-data",
                TelegramCommand::Remove { id: "abc".into(), delete_data: true },
            ),
            ("/rescan", TelegramCommand::Rescan),
            ("/reboot", TelegramCommand::Reboot),
        ];

        for (input, expected) in cases {
            assert_eq!(TelegramCommand::parse(input, Some("routerbot")).unwrap(), Some(expected));
        }
    }

    #[test]
    fn ignores_plain_text_and_commands_for_other_bots() {
        assert_eq!(TelegramCommand::parse("hello", Some("routerbot")).unwrap(), None);
        assert_eq!(TelegramCommand::parse("/help@other", Some("routerbot")).unwrap(), None);
    }

    #[test]
    fn rejects_invalid_arguments() {
        for input in ["/status now", "/add", "/stop a b", "/remove", "/remove a --force"] {
            assert!(TelegramCommand::parse(input, Some("routerbot")).is_err(), "{input}");
        }
    }

    #[test]
    fn normalizes_torrent_sources() {
        let magnet =
            TelegramCommand::Add("magnet:?xt=urn:btih:test".into()).into_bot_command().unwrap();
        let url = TelegramCommand::Add("https://example.com/file.torrent".into())
            .into_bot_command()
            .unwrap();

        assert!(matches!(magnet, BotCommand::AddTorrent { source: TorrentSource::Magnet(_) }));
        assert!(matches!(url, BotCommand::AddTorrent { source: TorrentSource::Url(_) }));
        assert!(
            TelegramCommand::Add("ftp://example.com/file.torrent".into())
                .into_bot_command()
                .is_err()
        );
    }
}
