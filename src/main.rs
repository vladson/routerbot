use anyhow::Context;
use routerbot_core::config::Config;
use routerbot_core::control_plane::{Command, CommandHandler, CommandSource, run_command_loop};
use routerbot_telegram::{TelegramControlPlane, build_bot};
use std::path::PathBuf;
use tokio::sync::mpsc;

struct LoggingHandler;

#[async_trait::async_trait]
impl CommandHandler for LoggingHandler {
    type Error = anyhow::Error;

    async fn handle(&mut self, command: Command) -> Result<(), Self::Error> {
        println!("Received command: {:?}", command);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::var("ROUTERBOT_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"));

    let config = Config::load_from_path(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;

    let (sender, receiver) = mpsc::channel(32);
    let bot = build_bot(&config.telegram.bot_token);
    let control_plane = TelegramControlPlane::new(bot);

    let control_plane_task = tokio::spawn(async move { control_plane.run(sender).await });
    let handler_task =
        tokio::spawn(async move { run_command_loop(receiver, LoggingHandler).await });

    tokio::select! {
        result = control_plane_task => {
            result.context("control plane task failed")??;
        }
        result = handler_task => {
            result.context("command handler task failed")??;
        }
    }

    Ok(())
}
