//! Integration coverage for Telegram command normalization and application routing.

use routerbot_app::{
    AppError, AuthzService, BoxFuture, CapabilityRegistry, CommandResponse, CommandService,
    DownloadControl,
};
use routerbot_chat_telegram::{TelegramCommand, format_command_response};
use routerbot_core::{DownloadId, Permission, TorrentSource};

struct MockDownloads;

impl DownloadControl for MockDownloads {
    fn add(&self, _source: TorrentSource) -> BoxFuture<'_, Result<DownloadId, AppError>> {
        Box::pin(async {
            DownloadId::new("hash-123").map_err(|error| AppError::CapabilityFailed {
                capability: routerbot_app::Capability::Downloads,
                message: error.to_string(),
            })
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<routerbot_core::DownloadStatus>, AppError>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn stop(&self, _id: DownloadId) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async { Ok(()) })
    }

    fn remove(&self, _id: DownloadId, _delete_data: bool) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn parsed_telegram_command_runs_through_application_service() {
    let service = CommandService::new(
        CapabilityRegistry::new().with_downloads(MockDownloads),
        AuthzService::allow_permissions([Permission::AddTorrent]),
    );
    let telegram_command =
        TelegramCommand::parse("/add magnet:?xt=urn:btih:example", Some("routerbot"))
            .expect("command should parse")
            .expect("input should contain a command");

    let response = service
        .execute(telegram_command.into_bot_command().expect("command should normalize"))
        .await
        .expect("command should execute");

    assert_eq!(
        response,
        CommandResponse::TorrentAdded(DownloadId::new("hash-123").expect("valid ID"))
    );
    assert_eq!(format_command_response(&response), "Torrent added: hash-123");
}
