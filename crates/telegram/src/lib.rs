use async_trait::async_trait;
use routerbot_core::control_plane::{Command, CommandSource};
use teloxide::prelude::*;
use tokio::sync::mpsc;

pub fn build_bot(token: &str) -> Bot {
    Bot::new(token)
}

pub struct TelegramControlPlane {
    bot: Bot,
}

impl TelegramControlPlane {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TelegramControlPlaneError {
    #[error("telegram request error: {0}")]
    Request(#[from] teloxide::RequestError),
}

#[async_trait]
impl CommandSource for TelegramControlPlane {
    type Error = TelegramControlPlaneError;

    async fn run(self, sender: mpsc::Sender<Command>) -> Result<(), Self::Error> {
        let handler = move |message: Message| {
            let sender = sender.clone();
            let bot = self.bot.clone();

            async move {
                if let Some(command) = parse_message(&message) {
                    match command {
                        ParsedCommand::App(command) => {
                            if sender.send(command).await.is_err() {
                                eprintln!("Command channel closed; dropping update.");
                            }
                        }
                        ParsedCommand::WhoAmI => {
                            if let Some(user) = message.from() {
                                let reply = format!("Your user id is {}", user.id);
                                if let Err(error) = bot.send_message(message.chat.id, reply).await {
                                    eprintln!("Failed to send reply: {error}");
                                }
                            }
                        }
                    }
                }

                respond(())
            }
        };

        teloxide::repl(self.bot, handler).await?;

        Ok(())
    }
}

fn parse_message(message: &Message) -> Option<ParsedCommand> {
    message.text().and_then(parse_text_command)
}

enum ParsedCommand {
    App(Command),
    WhoAmI,
}

fn parse_text_command(text: &str) -> Option<ParsedCommand> {
    let mut parts = text.split_whitespace();
    let command = parts.next()?;

    match command {
        "/start" => Some(ParsedCommand::App(Command::Start)),
        "/add" => {
            let url = parts.next()?;
            Some(ParsedCommand::App(Command::AddTorrent { url: url.to_string() }))
        }
        "/whoami" => Some(ParsedCommand::WhoAmI),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ParsedCommand, parse_text_command};

    #[test]
    fn parses_start_command() {
        assert!(matches!(
            parse_text_command("/start"),
            Some(ParsedCommand::App(super::Command::Start))
        ));
    }

    #[test]
    fn parses_add_command() {
        assert!(matches!(
            parse_text_command("/add http://example.com/torrent"),
            Some(ParsedCommand::App(super::Command::AddTorrent { .. }))
        ));
    }

    #[test]
    fn parses_whoami_command() {
        assert!(matches!(parse_text_command("/whoami"), Some(ParsedCommand::WhoAmI)));
    }

    #[test]
    fn ignores_unknown_command() {
        assert_eq!(parse_text_command("/ping"), None);
    }
}
