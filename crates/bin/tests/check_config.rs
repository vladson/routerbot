//! Integration coverage for runtime configuration validation.

use std::{path::PathBuf, process::Command};

fn packaged_config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/keenetic/config.yaml")
}

#[test]
fn checks_packaged_configuration_without_network_access() {
    let output = Command::new(env!("CARGO_BIN_EXE_routerbot"))
        .arg("--check-config")
        .arg("--config")
        .arg(packaged_config())
        .env("ROUTERBOT_TELEGRAM_TOKEN", "test-token")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Configuration is valid"));
}

#[test]
fn reads_configuration_path_from_environment() {
    let output = Command::new(env!("CARGO_BIN_EXE_routerbot"))
        .arg("--check-config")
        .env("ROUTERBOT_CONFIG", packaged_config())
        .env("ROUTERBOT_TELEGRAM_TOKEN", "test-token")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
