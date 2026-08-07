//! Integration tests for loading repository configuration examples from disk.

use std::path::PathBuf;

use routerbot_config::{AdapterConfig, RouterbotConfig};

#[test]
fn keenetic_example_loads_from_disk_with_milestone_capabilities() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/config.keenetic.yaml");

    let config = RouterbotConfig::from_path(path).expect("Keenetic example should load");

    assert!(matches!(
        config.adapters().get("transmission-main"),
        Some(AdapterConfig::TransmissionRpc { .. })
    ));
    assert!(matches!(
        config.adapters().get("keenetic-router"),
        Some(AdapterConfig::KeeneticLocal { ndmc_path }) if ndmc_path == &PathBuf::from("/bin/ndmc")
    ));
    assert_eq!(config.capabilities().downloads(), Some("transmission-main"));
    assert_eq!(config.capabilities().device(), Some("keenetic-router"));
}
