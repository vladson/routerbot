//! Routerbot binary entry point.

use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};
use routerbot_app::CapabilityRegistry;
#[cfg(feature = "telegram")]
use routerbot_app::{AuthzService, CommandService};
use routerbot_config::{AdapterConfig, RouterbotConfig};
use routerbot_core::Permission;

const DEFAULT_CONFIG_PATH: &str = "/opt/etc/routerbot/config.yaml";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let arguments = Arguments::parse(std::env::args_os().skip(1))?;
    if arguments.help {
        println!("{}", Arguments::usage());
        return Ok(());
    }
    if arguments.version {
        println!("routerbot {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config_path = arguments.config_path();
    let config = RouterbotConfig::from_path(&config_path)
        .with_context(|| format!("failed to load {}", config_path.display()))?;
    let secrets = config.resolve_secrets().context("failed to resolve configured secrets")?;
    let (registry, permissions) = build_registry(&config, &secrets)?;

    if arguments.check_config {
        println!("Configuration is valid: {}", config_path.display());
        return Ok(());
    }

    run_telegram(config, secrets, registry, permissions).await
}

#[cfg_attr(not(all(feature = "transmission", feature = "keenetic")), allow(unused_variables))]
fn build_registry(
    config: &RouterbotConfig,
    secrets: &routerbot_config::ResolvedSecrets,
) -> Result<(CapabilityRegistry, Vec<Permission>)> {
    #[allow(unused_mut)]
    let mut registry = CapabilityRegistry::new();
    #[allow(unused_mut)]
    let mut permissions = vec![Permission::ViewHelp, Permission::ViewStatus];

    if let Some(name) = config.capabilities().downloads() {
        let adapter =
            config.adapters().get(name).context("validated download adapter is missing")?;
        let AdapterConfig::TransmissionRpc { endpoint, .. } = adapter else {
            bail!("download capability {name} is not a Transmission RPC adapter");
        };
        #[cfg(feature = "transmission")]
        {
            use routerbot_adapter_transmission::{TransmissionCredentials, TransmissionRpcAdapter};

            let credentials = secrets.transmission(name).and_then(|values| {
                values.username().zip(values.password()).map(|(username, password)| {
                    TransmissionCredentials::new(username.expose(), password.expose())
                })
            });
            let adapter = TransmissionRpcAdapter::new(endpoint, credentials)
                .with_context(|| format!("failed to construct Transmission adapter {name}"))?;
            registry = registry.with_downloads(adapter);
            permissions.extend([
                Permission::ListDownloads,
                Permission::AddTorrent,
                Permission::StopDownload,
                Permission::RemoveDownload,
            ]);
        }
        #[cfg(not(feature = "transmission"))]
        bail!("download capability {name} requires the transmission feature");
    }

    if let Some(name) = config.capabilities().device() {
        let adapter = config.adapters().get(name).context("validated device adapter is missing")?;
        let AdapterConfig::KeeneticLocal { ndmc_path } = adapter else {
            bail!("device capability {name} is not a local Keenetic adapter");
        };
        #[cfg(feature = "keenetic")]
        {
            registry = registry
                .with_device(routerbot_adapter_keenetic::KeeneticLocalAdapter::new(ndmc_path));
            permissions.push(Permission::RebootDevice);
        }
        #[cfg(not(feature = "keenetic"))]
        bail!("device capability {name} requires the keenetic feature");
    }

    if let Some(name) = config.capabilities().media_index() {
        bail!("media index adapter {name} is not supported by this runtime yet");
    }
    if let Some(name) = config.capabilities().workload() {
        bail!("workload adapter {name} is not supported by this runtime yet");
    }

    Ok((registry, permissions))
}

#[cfg(feature = "telegram")]
async fn run_telegram(
    config: RouterbotConfig,
    secrets: routerbot_config::ResolvedSecrets,
    registry: CapabilityRegistry,
    permissions: Vec<Permission>,
) -> Result<()> {
    use routerbot_chat_telegram::{TelegramAccessPolicy, TelegramAdapter};

    let access_policy = TelegramAccessPolicy::new(
        config.auth().allow_chats().iter().copied(),
        config.auth().allow_users().iter().copied(),
    );
    let service = CommandService::new(registry, AuthzService::allow_permissions(permissions));
    TelegramAdapter::new(secrets.bot_token().expose(), service, access_policy)
        .run_long_polling()
        .await;
    Ok(())
}

#[cfg(not(feature = "telegram"))]
async fn run_telegram(
    _config: RouterbotConfig,
    _secrets: routerbot_config::ResolvedSecrets,
    _registry: CapabilityRegistry,
    _permissions: Vec<Permission>,
) -> Result<()> {
    bail!("the configured Telegram runtime requires the telegram feature")
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Arguments {
    config: Option<PathBuf>,
    check_config: bool,
    help: bool,
    version: bool,
}

impl Arguments {
    fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let mut parsed = Self::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--config") => {
                    let path = arguments.next().context("--config requires a path")?;
                    parsed.config = Some(PathBuf::from(path));
                }
                Some("--check-config") => parsed.check_config = true,
                Some("--help" | "-h") => parsed.help = true,
                Some("--version" | "-V") => parsed.version = true,
                Some(value) => bail!("unknown argument: {value}"),
                None => bail!("arguments must be valid Unicode, except for config paths"),
            }
        }
        Ok(parsed)
    }

    fn config_path(&self) -> PathBuf {
        self.config.clone().unwrap_or_else(|| {
            std::env::var_os("ROUTERBOT_CONFIG")
                .map_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH), PathBuf::from)
        })
    }

    const fn usage() -> &'static str {
        "Usage: routerbot [--config PATH] [--check-config] [--help] [--version]"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runtime_arguments() {
        let arguments = Arguments::parse([
            OsString::from("--config"),
            OsString::from("routerbot.yaml"),
            OsString::from("--check-config"),
        ])
        .unwrap();
        assert_eq!(arguments.config, Some(PathBuf::from("routerbot.yaml")));
        assert!(arguments.check_config);
    }

    #[test]
    fn rejects_unknown_argument() {
        assert!(Arguments::parse([OsString::from("--unknown")]).is_err());
    }
}
