//! Configuration loading and validation for Routerbot.
//!
//! This crate owns deployment configuration shape. It does not construct
//! application services, read secrets, or initialize adapters.

use std::{
    collections::{BTreeMap, HashSet},
    fmt, fs,
    path::Path,
    str::FromStr,
};

use serde::Deserialize;

/// Fully validated Routerbot configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterbotConfig {
    bot: BotConfig,
    #[serde(default)]
    auth: AuthConfig,
    #[serde(default)]
    adapters: AdapterConfigs,
    #[serde(default)]
    capabilities: CapabilityBindings,
}

impl RouterbotConfig {
    /// Parses and validates configuration from YAML text.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] when the YAML cannot be deserialized.
    /// Returns [`ConfigError::Validation`] when the deserialized configuration
    /// violates Routerbot's deployment rules.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &str) -> Result<Self, ConfigError> {
        input.parse()
    }

    /// Loads, parses, and validates configuration from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Read`] when the file cannot be read,
    /// [`ConfigError::Parse`] when the YAML cannot be deserialized, or
    /// [`ConfigError::Validation`] when validation fails.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let input = fs::read_to_string(path).map_err(ConfigError::Read)?;
        Self::from_str(&input)
    }

    /// Returns bot configuration.
    #[must_use]
    pub const fn bot(&self) -> &BotConfig {
        &self.bot
    }

    /// Returns authorization configuration.
    #[must_use]
    pub const fn auth(&self) -> &AuthConfig {
        &self.auth
    }

    /// Returns adapter configuration.
    #[must_use]
    pub const fn adapters(&self) -> &AdapterConfigs {
        &self.adapters
    }

    /// Returns capability binding configuration.
    #[must_use]
    pub const fn capabilities(&self) -> &CapabilityBindings {
        &self.capabilities
    }

    /// Resolves all configured secret environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::MissingEnvironment`] when a configured variable
    /// is absent or is not valid Unicode.
    pub fn resolve_secrets(&self) -> Result<ResolvedSecrets, ConfigError> {
        self.resolve_secrets_with(|name| std::env::var(name).ok())
    }

    fn resolve_secrets_with(
        &self,
        mut resolve: impl FnMut(&str) -> Option<String>,
    ) -> Result<ResolvedSecrets, ConfigError> {
        let bot_token = resolve_required_secret(&mut resolve, self.bot.token_env())?;
        let mut adapters = BTreeMap::new();
        for (name, adapter) in self.adapters.iter() {
            if let AdapterConfig::TransmissionRpc { username_env, password_env, .. } = adapter {
                let username = resolve_optional_secret(&mut resolve, username_env.as_deref())?;
                let password = resolve_optional_secret(&mut resolve, password_env.as_deref())?;
                adapters.insert(name.to_owned(), TransmissionSecrets { username, password });
            }
        }
        Ok(ResolvedSecrets { bot_token, transmission: adapters })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        self.bot.validate()?;
        self.auth.validate()?;
        self.adapters.validate()?;
        self.capabilities.validate(&self.adapters)
    }
}

impl FromStr for RouterbotConfig {
    type Err = ConfigError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let config = serde_yaml::from_str::<Self>(input).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }
}

/// Chat bot configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotConfig {
    kind: BotKind,
    token_env: String,
    mode: BotMode,
}

impl BotConfig {
    /// Returns the configured bot kind.
    #[must_use]
    pub const fn kind(&self) -> BotKind {
        self.kind
    }

    /// Returns the environment variable name containing the bot token.
    #[must_use]
    pub fn token_env(&self) -> &str {
        &self.token_env
    }

    /// Returns the configured bot runtime mode.
    #[must_use]
    pub const fn mode(&self) -> BotMode {
        self.mode
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.token_env.trim().is_empty() {
            return Err(ConfigError::validation("bot.token_env must not be empty"));
        }
        if !is_environment_variable_name(&self.token_env) {
            return Err(ConfigError::validation(
                "bot.token_env must be an environment variable name",
            ));
        }

        Ok(())
    }
}

/// Supported chat bot implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotKind {
    /// Telegram Bot API.
    Telegram,
}

/// Supported bot runtime modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotMode {
    /// Telegram long polling mode.
    LongPolling,
}

/// Authorization allow-list configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    #[serde(default)]
    allow_chats: Vec<i64>,
    #[serde(default)]
    allow_users: Vec<i64>,
}

impl AuthConfig {
    /// Returns allowed Telegram chat identifiers.
    #[must_use]
    pub fn allow_chats(&self) -> &[i64] {
        &self.allow_chats
    }

    /// Returns allowed Telegram user identifiers.
    #[must_use]
    pub fn allow_users(&self) -> &[i64] {
        &self.allow_users
    }

    fn validate(&self) -> Result<(), ConfigError> {
        reject_duplicates("auth.allow_chats", &self.allow_chats)?;
        reject_duplicates("auth.allow_users", &self.allow_users)
    }
}

/// Named adapter instance configurations.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(transparent)]
pub struct AdapterConfigs(BTreeMap<String, AdapterConfig>);

impl AdapterConfigs {
    /// Returns a configured adapter by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&AdapterConfig> {
        self.0.get(name)
    }

    /// Iterates over configured adapter names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &AdapterConfig)> {
        self.0.iter().map(|(name, config)| (name.as_str(), config))
    }

    fn validate(&self) -> Result<(), ConfigError> {
        for (name, adapter) in &self.0 {
            if name.trim().is_empty() {
                return Err(ConfigError::validation("adapter names must not be empty"));
            }
            adapter.validate(name)?;
        }
        Ok(())
    }
}

/// Configuration for a concrete adapter instance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum AdapterConfig {
    /// Transmission JSON-RPC endpoint.
    #[serde(rename = "transmission.rpc")]
    TransmissionRpc {
        /// Transmission RPC URL.
        endpoint: String,
        /// Optional environment variable containing the RPC username.
        #[serde(default)]
        username_env: Option<String>,
        /// Optional environment variable containing the RPC password.
        #[serde(default)]
        password_env: Option<String>,
    },
    /// DLNA service running locally on the Keenetic router.
    #[serde(rename = "keenetic.dlna")]
    KeeneticDlna,
}

/// Secret values resolved for runtime adapter construction.
pub struct ResolvedSecrets {
    bot_token: SecretValue,
    transmission: BTreeMap<String, TransmissionSecrets>,
}

impl ResolvedSecrets {
    /// Returns the Telegram bot token.
    #[must_use]
    pub const fn bot_token(&self) -> &SecretValue {
        &self.bot_token
    }

    /// Returns credentials for a named Transmission adapter.
    #[must_use]
    pub fn transmission(&self, name: &str) -> Option<&TransmissionSecrets> {
        self.transmission.get(name)
    }
}

impl fmt::Debug for ResolvedSecrets {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedSecrets")
            .field("bot_token", &self.bot_token)
            .field("transmission", &self.transmission)
            .finish()
    }
}

/// Resolved credentials for one Transmission adapter.
#[derive(Debug)]
pub struct TransmissionSecrets {
    username: Option<SecretValue>,
    password: Option<SecretValue>,
}

impl TransmissionSecrets {
    /// Returns the optional RPC username.
    #[must_use]
    pub const fn username(&self) -> Option<&SecretValue> {
        self.username.as_ref()
    }

    /// Returns the optional RPC password.
    #[must_use]
    pub const fn password(&self) -> Option<&SecretValue> {
        self.password.as_ref()
    }
}

/// A secret whose debug representation never reveals its value.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(String);

impl SecretValue {
    /// Exposes the secret for passing it to an external client.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl AdapterConfig {
    fn validate(&self, name: &str) -> Result<(), ConfigError> {
        match self {
            Self::TransmissionRpc { endpoint, username_env, password_env } => {
                if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
                    return Err(ConfigError::validation(format!(
                        "adapters.{name}.endpoint must be an HTTP or HTTPS URL"
                    )));
                }
                validate_optional_environment_name(
                    &format!("adapters.{name}.username_env"),
                    username_env.as_deref(),
                )?;
                validate_optional_environment_name(
                    &format!("adapters.{name}.password_env"),
                    password_env.as_deref(),
                )
            }
            Self::KeeneticDlna => Ok(()),
        }
    }
}

/// Capability-to-adapter binding configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityBindings {
    #[serde(default)]
    downloads: Option<String>,
    #[serde(default)]
    device: Option<String>,
    #[serde(default)]
    media_index: Option<String>,
    #[serde(default)]
    workload: Option<String>,
}

impl CapabilityBindings {
    /// Returns the download capability adapter binding.
    #[must_use]
    pub fn downloads(&self) -> Option<&str> {
        self.downloads.as_deref()
    }

    /// Returns the device capability adapter binding.
    #[must_use]
    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    /// Returns the media indexing capability adapter binding.
    #[must_use]
    pub fn media_index(&self) -> Option<&str> {
        self.media_index.as_deref()
    }

    /// Returns the workload capability adapter binding.
    #[must_use]
    pub fn workload(&self) -> Option<&str> {
        self.workload.as_deref()
    }

    fn validate(&self, adapters: &AdapterConfigs) -> Result<(), ConfigError> {
        reject_empty_binding("capabilities.downloads", self.downloads())?;
        reject_empty_binding("capabilities.device", self.device())?;
        reject_empty_binding("capabilities.media_index", self.media_index())?;
        reject_empty_binding("capabilities.workload", self.workload())?;

        validate_binding(adapters, "capabilities.downloads", self.downloads(), |adapter| {
            matches!(adapter, AdapterConfig::TransmissionRpc { .. })
        })?;
        validate_binding(adapters, "capabilities.media_index", self.media_index(), |adapter| {
            matches!(adapter, AdapterConfig::KeeneticDlna)
        })
    }
}

/// Configuration loading or validation error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A configuration file could not be read.
    #[error("failed to read configuration: {0}")]
    Read(std::io::Error),
    /// Configuration YAML could not be parsed.
    #[error("failed to parse configuration: {0}")]
    Parse(serde_yaml::Error),
    /// Configuration was parsed but failed validation.
    #[error("invalid configuration: {message}")]
    Validation {
        /// Human-readable validation failure.
        message: String,
    },
    /// A configured secret environment variable could not be resolved.
    #[error("required environment variable {name} is not set or is not valid Unicode")]
    MissingEnvironment {
        /// Environment variable name.
        name: String,
    },
}

impl ConfigError {
    fn validation(message: impl Into<String>) -> Self {
        Self::Validation { message: message.into() }
    }
}

fn resolve_required_secret(
    resolve: &mut impl FnMut(&str) -> Option<String>,
    name: &str,
) -> Result<SecretValue, ConfigError> {
    resolve(name)
        .map(SecretValue)
        .ok_or_else(|| ConfigError::MissingEnvironment { name: name.to_owned() })
}

fn resolve_optional_secret(
    resolve: &mut impl FnMut(&str) -> Option<String>,
    name: Option<&str>,
) -> Result<Option<SecretValue>, ConfigError> {
    name.map(|name| resolve_required_secret(resolve, name)).transpose()
}

fn reject_duplicates(field: &str, values: &[i64]) -> Result<(), ConfigError> {
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        if !seen.insert(value) {
            return Err(ConfigError::validation(format!(
                "{field} contains duplicate value {value}"
            )));
        }
    }
    Ok(())
}

fn reject_empty_binding(field: &str, value: Option<&str>) -> Result<(), ConfigError> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(ConfigError::validation(format!("{field} must not be empty")));
    }
    Ok(())
}

fn validate_optional_environment_name(field: &str, value: Option<&str>) -> Result<(), ConfigError> {
    if value.is_some_and(|value| !is_environment_variable_name(value)) {
        return Err(ConfigError::validation(format!(
            "{field} must be an environment variable name"
        )));
    }
    Ok(())
}

fn validate_binding(
    adapters: &AdapterConfigs,
    field: &str,
    binding: Option<&str>,
    supports_capability: impl FnOnce(&AdapterConfig) -> bool,
) -> Result<(), ConfigError> {
    let Some(binding) = binding else {
        return Ok(());
    };
    let adapter = adapters.get(binding).ok_or_else(|| {
        ConfigError::validation(format!("{field} references unknown adapter {binding}"))
    })?;
    if !supports_capability(adapter) {
        return Err(ConfigError::validation(format!(
            "{field} references incompatible adapter {binding}"
        )));
    }
    Ok(())
}

fn is_environment_variable_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|char| char == '_' || char.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CONFIG: &str = r"
bot:
  kind: telegram
  token_env: ROUTERBOT_TELEGRAM_TOKEN
  mode: long_polling

auth:
  allow_chats: [1001, 1002]
  allow_users: [2001]

adapters:
  transmission-main:
    kind: transmission.rpc
    endpoint: http://127.0.0.1:9091/transmission/rpc
    username_env: TRANSMISSION_USERNAME
    password_env: TRANSMISSION_PASSWORD
  keenetic-dlna:
    kind: keenetic.dlna
capabilities:
  downloads: transmission-main
  media_index: keenetic-dlna
";

    fn validation_message(error: ConfigError) -> String {
        match error {
            ConfigError::Validation { message } => message,
            other => panic!("expected validation error, got {other}"),
        }
    }

    #[test]
    fn valid_config_parses() {
        let config = RouterbotConfig::from_str(VALID_CONFIG).expect("config should be valid");

        assert_eq!(config.bot().kind(), BotKind::Telegram);
        assert_eq!(config.bot().mode(), BotMode::LongPolling);
        assert_eq!(config.bot().token_env(), "ROUTERBOT_TELEGRAM_TOKEN");
        assert_eq!(config.auth().allow_chats(), &[1001, 1002]);
        assert_eq!(config.auth().allow_users(), &[2001]);
        assert_eq!(config.capabilities().downloads(), Some("transmission-main"));
        assert_eq!(config.capabilities().device(), None);
        assert_eq!(config.capabilities().media_index(), Some("keenetic-dlna"));
        assert_eq!(config.capabilities().workload(), None);
    }

    #[test]
    fn unknown_bot_kind_fails() {
        let input = VALID_CONFIG.replace("kind: telegram", "kind: discord");

        assert!(matches!(RouterbotConfig::from_str(&input), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn unknown_bot_mode_fails() {
        let input = VALID_CONFIG.replace("mode: long_polling", "mode: webhook");

        assert!(matches!(RouterbotConfig::from_str(&input), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn empty_token_env_fails() {
        let input = VALID_CONFIG.replace("token_env: ROUTERBOT_TELEGRAM_TOKEN", "token_env: '   '");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "bot.token_env must not be empty");
    }

    #[test]
    fn token_env_must_be_environment_variable_name() {
        let input =
            VALID_CONFIG.replace("token_env: ROUTERBOT_TELEGRAM_TOKEN", "token_env: token-value");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "bot.token_env must be an environment variable name");
    }

    #[test]
    fn duplicate_chat_ids_fail() {
        let input = VALID_CONFIG.replace("allow_chats: [1001, 1002]", "allow_chats: [1001, 1001]");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "auth.allow_chats contains duplicate value 1001");
    }

    #[test]
    fn duplicate_user_ids_fail() {
        let input = VALID_CONFIG.replace("allow_users: [2001]", "allow_users: [2001, 2001]");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "auth.allow_users contains duplicate value 2001");
    }

    #[test]
    fn empty_capability_binding_fails() {
        let input = VALID_CONFIG.replace("downloads: transmission-main", "downloads: '   '");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "capabilities.downloads must not be empty");
    }

    #[test]
    fn unknown_capability_adapter_fails() {
        let input = VALID_CONFIG.replace("downloads: transmission-main", "downloads: missing");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "capabilities.downloads references unknown adapter missing");
    }

    #[test]
    fn incompatible_capability_adapter_fails() {
        let input =
            VALID_CONFIG.replace("downloads: transmission-main", "downloads: keenetic-dlna");

        let message = validation_message(
            RouterbotConfig::from_str(&input).expect_err("config should be invalid"),
        );

        assert_eq!(message, "capabilities.downloads references incompatible adapter keenetic-dlna");
    }

    #[test]
    fn secrets_are_resolved_and_redacted() {
        let config = RouterbotConfig::from_str(VALID_CONFIG).expect("config should be valid");
        let secrets = config
            .resolve_secrets_with(|name| Some(format!("secret-for-{name}")))
            .expect("secrets should resolve");

        assert_eq!(secrets.bot_token().expose(), "secret-for-ROUTERBOT_TELEGRAM_TOKEN");
        let transmission = secrets
            .transmission("transmission-main")
            .expect("Transmission credentials should exist");
        assert_eq!(
            transmission.password().expect("password should exist").expose(),
            "secret-for-TRANSMISSION_PASSWORD"
        );
        let debug = format!("{secrets:?}");
        assert!(!debug.contains("secret-for-"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn missing_secret_fails_with_variable_name() {
        let config = RouterbotConfig::from_str(VALID_CONFIG).expect("config should be valid");

        let error =
            config.resolve_secrets_with(|_| None).expect_err("secret resolution should fail");

        assert!(matches!(
            error,
            ConfigError::MissingEnvironment { name }
                if name == "ROUTERBOT_TELEGRAM_TOKEN"
        ));
    }

    #[test]
    fn malformed_yaml_fails() {
        assert!(matches!(RouterbotConfig::from_str("bot: ["), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn missing_optional_sections_default() {
        let input = r"
bot:
  kind: telegram
  token_env: ROUTERBOT_TELEGRAM_TOKEN
  mode: long_polling
";

        let config = RouterbotConfig::from_str(input).expect("config should be valid");

        assert!(config.auth().allow_chats().is_empty());
        assert!(config.auth().allow_users().is_empty());
        assert_eq!(config.capabilities().downloads(), None);
    }

    #[test]
    fn example_configs_parse() {
        for input in [
            include_str!("../../../examples/config.linux.yaml"),
            include_str!("../../../examples/config.keenetic.yaml"),
            include_str!("../../../examples/config.k3s.yaml"),
        ] {
            RouterbotConfig::from_str(input).expect("example config should be valid");
        }
    }
}
