use async_trait::async_trait;
use tokio::sync::mpsc;

/// Control plane commands emitted by transports such as Telegram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Start,
    AddTorrent { url: String },
}

/// A source of commands (control plane transport).
#[async_trait]
pub trait CommandSource {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn run(self, sender: mpsc::Sender<Command>) -> Result<(), Self::Error>;
}

/// A handler for control plane commands.
#[async_trait]
pub trait CommandHandler {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn handle(&mut self, command: Command) -> Result<(), Self::Error>;
}

/// Consume commands from the receiver and process them with the handler.
pub async fn run_command_loop<H: CommandHandler>(
    mut receiver: mpsc::Receiver<Command>,
    mut handler: H,
) -> Result<(), H::Error> {
    while let Some(command) = receiver.recv().await {
        handler.handle(command).await?;
    }

    Ok(())
}
