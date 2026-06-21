//! Command execution service.

use routerbot_core::{BotCommand, DownloadId, DownloadStatus, MediaRescanResult};

use crate::{AppError, AuthzService, CapabilityRegistry};

/// Result of executing a normalized command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResponse {
    /// Help was requested.
    Help,
    /// High-level status was requested.
    Status,
    /// Current downloads.
    Downloads(Vec<DownloadStatus>),
    /// A torrent was added.
    TorrentAdded(DownloadId),
    /// A download was stopped.
    DownloadStopped(DownloadId),
    /// Media indexing result.
    MediaRescan(MediaRescanResult),
    /// Device reboot was started.
    DeviceRebootStarted,
}

/// Executes normalized commands through authorization and capability routing.
#[derive(Clone)]
pub struct CommandService {
    registry: CapabilityRegistry,
    authz: AuthzService,
}

impl CommandService {
    /// Creates a command service.
    #[must_use]
    pub const fn new(registry: CapabilityRegistry, authz: AuthzService) -> Self {
        Self { registry, authz }
    }

    /// Executes a normalized command.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Unauthorized`] when authorization denies the command.
    /// Returns [`AppError::MissingCapability`] when a required capability is not
    /// configured. Returns [`AppError::CapabilityFailed`] when a configured
    /// capability reports a failure.
    pub async fn execute(&self, command: BotCommand) -> Result<CommandResponse, AppError> {
        self.authz.authorize(&command)?;

        match command {
            BotCommand::Help => Ok(CommandResponse::Help),
            BotCommand::ShowStatus => Ok(CommandResponse::Status),
            BotCommand::ListDownloads => {
                let downloads = self.registry.downloads()?.list().await?;
                Ok(CommandResponse::Downloads(downloads))
            }
            BotCommand::AddTorrent { source } => {
                let id = self.registry.downloads()?.add(source).await?;
                Ok(CommandResponse::TorrentAdded(id))
            }
            BotCommand::StopDownload { id } => {
                self.registry.downloads()?.stop(id.clone()).await?;
                Ok(CommandResponse::DownloadStopped(id))
            }
            BotCommand::RescanMedia => {
                let result = self.registry.media_index()?.rescan().await?;
                Ok(CommandResponse::MediaRescan(result))
            }
            BotCommand::RebootDevice => {
                self.registry.device()?.reboot().await?;
                Ok(CommandResponse::DeviceRebootStarted)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Poll, Waker},
    };

    use routerbot_core::{
        DownloadState, Permission, TorrentSource, WorkloadKind, WorkloadStatus, WorkloadTarget,
    };

    use super::*;
    use crate::{
        BoxFuture, Capability, DeviceControl, DownloadControl, MediaIndexControl, WorkloadControl,
    };

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = Pin::from(Box::new(future));

        loop {
            match Future::poll(future.as_mut(), &mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn sample_download_id() -> DownloadId {
        DownloadId::new("download-1").expect("download id should be valid")
    }

    fn sample_download_status() -> DownloadStatus {
        DownloadStatus::new(
            sample_download_id(),
            "Ubuntu ISO",
            DownloadState::Downloading,
            50,
            Some(1024),
            None,
        )
        .expect("download status should be valid")
    }

    #[derive(Default)]
    struct MockDownloads {
        stop_called: Arc<AtomicBool>,
    }

    impl DownloadControl for MockDownloads {
        fn add(&self, _source: TorrentSource) -> BoxFuture<'_, Result<DownloadId, AppError>> {
            Box::pin(async { Ok(sample_download_id()) })
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<DownloadStatus>, AppError>> {
            Box::pin(async { Ok(vec![sample_download_status()]) })
        }

        fn stop(&self, _id: DownloadId) -> BoxFuture<'_, Result<(), AppError>> {
            let stop_called = Arc::clone(&self.stop_called);
            Box::pin(async move {
                stop_called.store(true, Ordering::SeqCst);
                Ok(())
            })
        }
    }

    struct MockDevice;

    impl DeviceControl for MockDevice {
        fn reboot(&self) -> BoxFuture<'_, Result<(), AppError>> {
            Box::pin(async { Ok(()) })
        }

        fn uptime(&self) -> BoxFuture<'_, Result<Option<std::time::Duration>, AppError>> {
            Box::pin(async { Ok(None) })
        }
    }

    struct MockMediaIndex;

    impl MediaIndexControl for MockMediaIndex {
        fn rescan(&self) -> BoxFuture<'_, Result<MediaRescanResult, AppError>> {
            Box::pin(async { Ok(MediaRescanResult::triggered("rescan started")) })
        }
    }

    struct MockWorkload;

    impl WorkloadControl for MockWorkload {
        fn restart_workload(&self, _target: WorkloadTarget) -> BoxFuture<'_, Result<(), AppError>> {
            Box::pin(async { Ok(()) })
        }

        fn workload_status(
            &self,
            target: WorkloadTarget,
        ) -> BoxFuture<'_, Result<WorkloadStatus, AppError>> {
            Box::pin(async move {
                Ok(WorkloadStatus {
                    target,
                    desired_replicas: Some(1),
                    ready_replicas: Some(1),
                    available: true,
                    message: None,
                })
            })
        }
    }

    fn service_with_all_permissions(registry: CapabilityRegistry) -> CommandService {
        CommandService::new(
            registry,
            AuthzService::allow_permissions([
                Permission::ViewHelp,
                Permission::ViewStatus,
                Permission::ListDownloads,
                Permission::AddTorrent,
                Permission::StopDownload,
                Permission::RescanMedia,
                Permission::RebootDevice,
            ]),
        )
    }

    #[test]
    fn authz_denies_by_default() {
        let authz = AuthzService::deny_all();

        assert!(!authz.is_allowed(&BotCommand::Help));
        assert_eq!(
            authz.authorize(&BotCommand::Help),
            Err(AppError::Unauthorized { permission: Permission::ViewHelp })
        );
    }

    #[test]
    fn authz_allows_configured_permissions() {
        let authz = AuthzService::allow_permissions([Permission::ViewHelp]);

        assert!(authz.is_allowed(&BotCommand::Help));
        assert!(!authz.is_allowed(&BotCommand::RebootDevice));
    }

    #[test]
    fn command_service_rejects_denied_command_before_capability_lookup() {
        let service = CommandService::new(CapabilityRegistry::new(), AuthzService::deny_all());

        let result = block_on(service.execute(BotCommand::ListDownloads));

        assert_eq!(result, Err(AppError::Unauthorized { permission: Permission::ListDownloads }));
    }

    #[test]
    fn command_service_returns_missing_capability() {
        let service = service_with_all_permissions(CapabilityRegistry::new());

        let result = block_on(service.execute(BotCommand::ListDownloads));

        assert_eq!(result, Err(AppError::MissingCapability { capability: Capability::Downloads }));
    }

    #[test]
    fn command_service_routes_download_list() {
        let service = service_with_all_permissions(
            CapabilityRegistry::new().with_downloads(MockDownloads::default()),
        );

        let response = block_on(service.execute(BotCommand::ListDownloads))
            .expect("download list should succeed");

        match response {
            CommandResponse::Downloads(downloads) => {
                assert_eq!(downloads.len(), 1);
                assert_eq!(downloads[0].name(), "Ubuntu ISO");
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn command_service_routes_add_torrent() {
        let service = service_with_all_permissions(
            CapabilityRegistry::new().with_downloads(MockDownloads::default()),
        );

        let response = block_on(service.execute(BotCommand::AddTorrent {
            source: TorrentSource::Magnet("magnet:?xt=urn:btih:example".to_owned()),
        }))
        .expect("add torrent should succeed");

        assert_eq!(response, CommandResponse::TorrentAdded(sample_download_id()));
    }

    #[test]
    fn command_service_routes_stop_download() {
        let downloads = MockDownloads::default();
        let stop_called = Arc::clone(&downloads.stop_called);
        let service =
            service_with_all_permissions(CapabilityRegistry::new().with_downloads(downloads));

        let response =
            block_on(service.execute(BotCommand::StopDownload { id: sample_download_id() }))
                .expect("stop download should succeed");

        assert_eq!(response, CommandResponse::DownloadStopped(sample_download_id()));
        assert!(stop_called.load(Ordering::SeqCst));
    }

    #[test]
    fn command_service_routes_media_rescan() {
        let service = service_with_all_permissions(
            CapabilityRegistry::new().with_media_index(MockMediaIndex),
        );

        let response =
            block_on(service.execute(BotCommand::RescanMedia)).expect("rescan should succeed");

        assert_eq!(
            response,
            CommandResponse::MediaRescan(MediaRescanResult::triggered("rescan started"))
        );
    }

    #[test]
    fn command_service_routes_device_reboot() {
        let service =
            service_with_all_permissions(CapabilityRegistry::new().with_device(MockDevice));

        let response =
            block_on(service.execute(BotCommand::RebootDevice)).expect("reboot should succeed");

        assert_eq!(response, CommandResponse::DeviceRebootStarted);
    }

    #[test]
    fn registry_exposes_workload_capability() {
        let registry = CapabilityRegistry::new().with_workload(MockWorkload);
        let workload = registry.workload().expect("workload should be configured");
        let target =
            WorkloadTarget::new(WorkloadKind::Deployment, Some("media".to_owned()), "minidlna")
                .expect("workload target should be valid");

        let status =
            block_on(workload.workload_status(target)).expect("workload status should succeed");

        assert!(status.available);
        assert_eq!(status.target.name(), "minidlna");
    }
}
