//! Local Keenetic device control through `ndmc`.

use std::{path::PathBuf, process::Stdio, time::Duration};

use routerbot_app::{AppError, BoxFuture, Capability, DeviceControl};
use tokio::{process::Command, time::timeout};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_DIAGNOSTIC_BYTES: usize = 512;

/// Device control backed by the router-local Keenetic CLI.
#[derive(Debug, Clone)]
pub struct KeeneticLocalAdapter {
    ndmc_path: PathBuf,
}

impl KeeneticLocalAdapter {
    /// Creates a local Keenetic adapter using the given `ndmc` executable.
    #[must_use]
    pub fn new(ndmc_path: impl Into<PathBuf>) -> Self {
        Self { ndmc_path: ndmc_path.into() }
    }

    async fn execute(&self, command: &'static str) -> Result<String, AppError> {
        let child = Command::new(&self.ndmc_path)
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| capability_error(format!("failed to start ndmc: {error}")))?;

        let output = timeout(COMMAND_TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| capability_error("ndmc command timed out"))?
            .map_err(|error| capability_error(format!("failed to wait for ndmc: {error}")))?;
        if !output.status.success() {
            let diagnostic = diagnostic(&output.stderr, &output.stdout);
            let suffix =
                if diagnostic.is_empty() { String::new() } else { format!(": {diagnostic}") };
            return Err(capability_error(format!("ndmc exited with {}{suffix}", output.status)));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| capability_error("ndmc returned non-UTF-8 output"))
    }
}

impl Default for KeeneticLocalAdapter {
    fn default() -> Self {
        Self::new("/bin/ndmc")
    }
}

impl DeviceControl for KeeneticLocalAdapter {
    fn reboot(&self) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            self.execute("system reboot").await?;
            Ok(())
        })
    }

    fn uptime(&self) -> BoxFuture<'_, Result<Option<Duration>, AppError>> {
        Box::pin(async move {
            let output = self.execute("show system").await?;
            parse_uptime(&output).map(Some)
        })
    }
}

fn parse_uptime(output: &str) -> Result<Duration, AppError> {
    let value = output.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "uptime").then(|| value.trim())
    });
    let value = value.ok_or_else(|| capability_error("ndmc output does not contain uptime"))?;
    let seconds = value
        .parse::<u64>()
        .map_err(|_| capability_error("ndmc returned an invalid uptime value"))?;
    Ok(Duration::from_secs(seconds))
}

fn diagnostic(primary: &[u8], fallback: &[u8]) -> String {
    let bytes = if primary.is_empty() { fallback } else { primary };
    let end = bytes.len().min(MAX_DIAGNOSTIC_BYTES);
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

fn capability_error(message: impl Into<String>) -> AppError {
    AppError::CapabilityFailed { capability: Capability::Device, message: message.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_router_uptime() {
        let output = "         hostname: Keenetic_Viva\n           uptime: 62055\n";
        assert_eq!(parse_uptime(output).unwrap(), Duration::from_secs(62_055));
    }

    #[test]
    fn rejects_missing_uptime() {
        assert!(parse_uptime("hostname: router").is_err());
    }

    #[test]
    fn rejects_invalid_uptime() {
        assert!(parse_uptime("uptime: unknown").is_err());
    }
}
