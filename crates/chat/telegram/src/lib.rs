//! Telegram chat adapter for Routerbot.
//!
//! The adapter parses Telegram commands, enforces chat and user allow lists,
//! obtains confirmation for dangerous actions, and delegates execution to the
//! application layer.

mod access;
mod command;
mod confirmation;
mod format;
mod runtime;

pub use access::TelegramAccessPolicy;
pub use command::{TelegramCommand, TelegramCommandParseError};
pub use format::format_command_response;
pub use runtime::TelegramAdapter;
