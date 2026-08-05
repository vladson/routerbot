//! Transmission RPC 16 adapter for Routerbot.
//!
//! The first Keenetic milestone targets Transmission 3.00 and its legacy RPC
//! protocol. Protocol-specific types remain contained in this crate.

use std::{fmt::Display, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use routerbot_app::{AppError, BoxFuture, Capability, DownloadControl};
use routerbot_core::{DownloadId, DownloadState, DownloadStatus, TorrentSource};
use transmission_rpc::{
    SharableTransClient,
    types::{
        ErrorType, Id, Torrent, TorrentAction, TorrentAddArgs, TorrentAddedOrDuplicate,
        TorrentGetField, TorrentStatus,
    },
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Credentials used for Transmission HTTP Basic authentication.
///
/// This type intentionally does not implement [`Debug`](std::fmt::Debug) so a
/// password cannot be emitted accidentally.
pub struct TransmissionCredentials {
    username: String,
    password: String,
}

impl TransmissionCredentials {
    /// Creates Transmission RPC credentials.
    #[must_use]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self { username: username.into(), password: password.into() }
    }
}

/// Error produced while constructing a Transmission adapter.
#[derive(Debug, thiserror::Error)]
pub enum TransmissionAdapterError {
    /// The configured endpoint is not a valid URL.
    #[error("invalid Transmission RPC endpoint: {message}")]
    InvalidEndpoint {
        /// URL parser failure.
        message: String,
    },
    /// The configured username cannot be represented by HTTP Basic authentication.
    #[error("Transmission username must not contain ':'")]
    InvalidUsername,
    /// The generated authorization header is invalid.
    #[error("invalid Transmission authorization header: {0}")]
    InvalidAuthorizationHeader(#[from] reqwest::header::InvalidHeaderValue),
    /// The HTTP client could not be constructed.
    #[error("failed to construct Transmission HTTP client: {0}")]
    HttpClient(#[from] reqwest::Error),
}

/// Download control backed by a Transmission RPC endpoint.
pub struct TransmissionRpcAdapter {
    client: SharableTransClient,
}

impl TransmissionRpcAdapter {
    /// Creates an adapter for a Transmission RPC endpoint.
    ///
    /// Credentials are installed as a sensitive HTTP header instead of being
    /// stored in `transmission-rpc`'s authentication field. This prevents that
    /// dependency's debug logging from rendering the password.
    ///
    /// # Errors
    ///
    /// Returns [`TransmissionAdapterError`] when the endpoint, credentials, or
    /// HTTP client configuration is invalid.
    pub fn new(
        endpoint: &str,
        credentials: Option<TransmissionCredentials>,
    ) -> Result<Self, TransmissionAdapterError> {
        let endpoint = Url::parse(endpoint).map_err(|error| {
            TransmissionAdapterError::InvalidEndpoint { message: error.to_string() }
        })?;
        let mut headers = HeaderMap::new();
        if let Some(credentials) = credentials {
            if credentials.username.contains(':') {
                return Err(TransmissionAdapterError::InvalidUsername);
            }
            let encoded =
                STANDARD.encode(format!("{}:{}", credentials.username, credentials.password));
            let mut authorization = HeaderValue::from_str(&format!("Basic {encoded}"))?;
            authorization.set_sensitive(true);
            headers.insert(AUTHORIZATION, authorization);
        }
        let http_client =
            Client::builder().default_headers(headers).timeout(REQUEST_TIMEOUT).build()?;

        Ok(Self { client: SharableTransClient::new_with_client(endpoint, http_client) })
    }
}

impl DownloadControl for TransmissionRpcAdapter {
    fn add(&self, source: TorrentSource) -> BoxFuture<'_, Result<DownloadId, AppError>> {
        Box::pin(async move {
            let filename = match source {
                TorrentSource::Magnet(value) | TorrentSource::Url(value) => value,
                TorrentSource::TorrentFileName(_) => {
                    return Err(capability_error(
                        "adding uploaded torrent files is not supported yet",
                    ));
                }
            };
            let response = self
                .client
                .torrent_add(TorrentAddArgs {
                    filename: Some(filename),
                    ..TorrentAddArgs::default()
                })
                .await
                .map_err(capability_error)?;
            ensure_success(&response.result)?;

            let torrent = match response.arguments {
                TorrentAddedOrDuplicate::TorrentAdded(torrent)
                | TorrentAddedOrDuplicate::TorrentDuplicate(torrent) => torrent,
                TorrentAddedOrDuplicate::Error => {
                    return Err(capability_error(
                        "Transmission returned neither an added nor duplicate torrent",
                    ));
                }
            };
            torrent_download_id(&torrent)
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<DownloadStatus>, AppError>> {
        Box::pin(async move {
            let response = self
                .client
                .torrent_get(
                    Some(vec![
                        TorrentGetField::Error,
                        TorrentGetField::HashString,
                        TorrentGetField::Name,
                        TorrentGetField::PercentDone,
                        TorrentGetField::RateDownload,
                        TorrentGetField::RateUpload,
                        TorrentGetField::Status,
                    ]),
                    None,
                )
                .await
                .map_err(capability_error)?;
            ensure_success(&response.result)?;

            response.arguments.torrents.iter().map(map_torrent).collect()
        })
    }

    fn stop(&self, id: DownloadId) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let response = self
                .client
                .torrent_action(TorrentAction::Stop, vec![Id::Hash(id.as_str().to_owned())])
                .await
                .map_err(capability_error)?;
            ensure_success(&response.result)
        })
    }

    fn remove(&self, id: DownloadId, delete_data: bool) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let response = self
                .client
                .torrent_remove(vec![Id::Hash(id.as_str().to_owned())], delete_data)
                .await
                .map_err(capability_error)?;
            ensure_success(&response.result)
        })
    }
}

fn ensure_success(result: &str) -> Result<(), AppError> {
    if result == "success" {
        Ok(())
    } else {
        Err(capability_error(format!("Transmission RPC failed: {result}")))
    }
}

fn torrent_download_id(torrent: &Torrent) -> Result<DownloadId, AppError> {
    let value = torrent
        .hash_string
        .as_deref()
        .map(str::to_owned)
        .ok_or_else(|| capability_error("Transmission response omitted hashString"))?;
    DownloadId::new(value).map_err(capability_error)
}

fn map_torrent(torrent: &Torrent) -> Result<DownloadStatus, AppError> {
    let id = torrent_download_id(torrent)?;
    let name = required(torrent.name.as_deref(), "name")?;
    let progress = progress_percent(torrent.percent_done)?;
    let state = map_state(torrent.status, torrent.error, progress)?;
    let download_rate = non_negative_rate(torrent.rate_download, "rateDownload")?;
    let upload_rate = non_negative_rate(torrent.rate_upload, "rateUpload")?;

    DownloadStatus::new(id, name, state, progress, download_rate, upload_rate)
        .map_err(capability_error)
}

fn required<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, AppError> {
    value.ok_or_else(|| capability_error(format!("Transmission response omitted {field}")))
}

fn progress_percent(value: Option<f32>) -> Result<u8, AppError> {
    let value =
        value.ok_or_else(|| capability_error("Transmission response omitted percentDone"))?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(capability_error(format!("Transmission returned invalid percentDone {value}")));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let percent = (value * 100.0).round() as u8;
    Ok(percent)
}

fn non_negative_rate(value: Option<i64>, field: &str) -> Result<Option<u64>, AppError> {
    value
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                capability_error(format!("Transmission returned negative {field} {value}"))
            })
        })
        .transpose()
}

fn map_state(
    status: Option<TorrentStatus>,
    error: Option<ErrorType>,
    progress: u8,
) -> Result<DownloadState, AppError> {
    if error.is_some_and(|error| error != ErrorType::Ok) {
        return Ok(DownloadState::Error);
    }
    let status = status.ok_or_else(|| capability_error("Transmission response omitted status"))?;
    Ok(match status {
        TorrentStatus::Stopped if progress == 100 => DownloadState::Completed,
        TorrentStatus::Stopped => DownloadState::Stopped,
        TorrentStatus::QueuedToVerify
        | TorrentStatus::Verifying
        | TorrentStatus::QueuedToDownload
        | TorrentStatus::QueuedToSeed => DownloadState::Queued,
        TorrentStatus::Downloading => DownloadState::Downloading,
        TorrentStatus::Seeding => DownloadState::Seeding,
    })
}

fn capability_error(error: impl Display) -> AppError {
    AppError::CapabilityFailed { capability: Capability::Downloads, message: error.to_string() }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef01234567";

    #[tokio::test]
    async fn add_retries_with_session_id_and_returns_hash() {
        let (endpoint, server) = rpc_server(&format!(
            r#"{{"arguments":{{"torrent-added":{{"hashString":"{HASH}","id":7,"name":"Router image"}}}},"result":"success"}}"#
        ));
        let adapter = TransmissionRpcAdapter::new(
            &endpoint,
            Some(TransmissionCredentials::new("routerbot", "top-secret")),
        )
        .expect("adapter should be constructed");

        let id = adapter
            .add(TorrentSource::Magnet("magnet:?xt=urn:btih:test".to_owned()))
            .await
            .expect("torrent should be added");
        let requests = server.join().expect("mock server should finish");

        assert_eq!(id.as_str(), HASH);
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains(r#""method":"torrent-add""#));
        assert!(requests[0].contains(r#""filename":"magnet:?xt=urn:btih:test""#));
        assert!(requests[0].contains("authorization: basic "));
        assert!(!requests[0].contains("top-secret"));
        assert!(requests[1].contains("x-transmission-session-id: test-session"));
    }

    #[tokio::test]
    async fn add_rejects_numeric_id_without_stable_hash() {
        let (endpoint, server) = rpc_server(
            r#"{"arguments":{"torrent-added":{"id":7,"name":"Router image"}},"result":"success"}"#,
        );
        let adapter =
            TransmissionRpcAdapter::new(&endpoint, None).expect("adapter should be constructed");

        let error = adapter
            .add(TorrentSource::Magnet("magnet:?xt=urn:btih:test".to_owned()))
            .await
            .expect_err("numeric id must not replace a stable hash");
        server.join().expect("mock server should finish");

        assert_eq!(
            error,
            AppError::CapabilityFailed {
                capability: Capability::Downloads,
                message: "Transmission response omitted hashString".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn list_maps_transmission_statuses() {
        let body = format!(
            r#"{{"arguments":{{"torrents":[{{"error":0,"hashString":"{HASH}","name":"Router image","percentDone":0.42,"rateDownload":1024,"rateUpload":12,"status":4}}]}},"result":"success"}}"#
        );
        let (endpoint, server) = rpc_server(&body);
        let adapter =
            TransmissionRpcAdapter::new(&endpoint, None).expect("adapter should be constructed");

        let downloads = adapter.list().await.expect("downloads should be listed");
        let requests = server.join().expect("mock server should finish");

        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].id().as_str(), HASH);
        assert_eq!(downloads[0].state(), DownloadState::Downloading);
        assert_eq!(downloads[0].progress_percent(), 42);
        assert_eq!(downloads[0].download_rate_bytes_per_second(), Some(1024));
        assert!(requests[0].contains(r#""method":"torrent-get""#));
        assert!(requests[0].contains(r#""hashstring""#));
    }

    #[tokio::test]
    async fn stop_uses_stable_hash_identifier() {
        let (endpoint, server) = rpc_server(r#"{"arguments":{},"result":"success"}"#);
        let adapter =
            TransmissionRpcAdapter::new(&endpoint, None).expect("adapter should be constructed");
        let id = DownloadId::new(HASH).expect("hash should be a valid download id");

        adapter.stop(id).await.expect("torrent should be stopped");
        let requests = server.join().expect("mock server should finish");

        assert!(requests[0].contains(r#""method":"torrent-stop""#));
        assert!(requests[0].contains(&format!(r#""ids":["{HASH}"]"#)));
    }

    #[tokio::test]
    async fn remove_controls_local_data_deletion_explicitly() {
        for delete_data in [false, true] {
            let (endpoint, server) = rpc_server(r#"{"arguments":{},"result":"success"}"#);
            let adapter = TransmissionRpcAdapter::new(&endpoint, None)
                .expect("adapter should be constructed");
            let id = DownloadId::new(HASH).expect("hash should be a valid download id");

            adapter.remove(id, delete_data).await.expect("torrent should be removed");
            let requests = server.join().expect("mock server should finish");

            assert!(requests[0].contains(r#""method":"torrent-remove""#));
            assert!(requests[0].contains(&format!(r#""ids":["{HASH}"]"#)));
            assert!(requests[0].contains(&format!(r#""delete-local-data":{delete_data}"#)));
        }
    }

    #[tokio::test]
    async fn uploaded_torrent_file_is_rejected_before_rpc() {
        let adapter = TransmissionRpcAdapter::new("http://127.0.0.1:1/transmission/rpc", None)
            .expect("adapter should be constructed");

        let result = adapter.add(TorrentSource::TorrentFileName("upload.torrent".to_owned())).await;

        assert!(matches!(result, Err(AppError::CapabilityFailed { .. })));
    }

    fn rpc_server(success_body: &str) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock listener should bind");
        let address = listener.local_addr().expect("mock address should be available");
        let success_body = success_body.to_owned();
        let server = thread::spawn(move || {
            let mut requests = Vec::with_capacity(2);
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().expect("mock request should arrive");
                requests.push(read_http_request(&mut stream));
                if attempt == 0 {
                    stream
                        .write_all(
                            b"HTTP/1.1 409 Conflict\r\nX-Transmission-Session-Id: test-session\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .expect("session response should be written");
                } else {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        success_body.len(),
                        success_body
                    );
                    stream
                        .write_all(response.as_bytes())
                        .expect("success response should be written");
                }
            }
            requests
        });
        (format!("http://{address}/transmission/rpc"), server)
    }

    fn read_http_request(stream: &mut impl Read) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        let header_end = loop {
            let count = stream.read(&mut buffer).expect("mock request should be readable");
            assert_ne!(count, 0, "request ended before headers were complete");
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]).to_ascii_lowercase();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .map(str::trim)
            .map_or(0, |value| value.parse::<usize>().expect("content length should be numeric"));
        while bytes.len() < header_end + content_length {
            let count = stream.read(&mut buffer).expect("mock body should be readable");
            assert_ne!(count, 0, "request ended before body was complete");
            bytes.extend_from_slice(&buffer[..count]);
        }
        String::from_utf8(bytes).expect("mock request should be UTF-8").to_ascii_lowercase()
    }
}
