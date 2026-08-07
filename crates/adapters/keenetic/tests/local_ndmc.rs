//! Integration coverage for the local `ndmc` process boundary.

use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use routerbot_adapter_keenetic::KeeneticLocalAdapter;
use routerbot_app::DeviceControl;

#[tokio::test]
async fn invokes_ndmc_and_parses_uptime() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("ndmc");
    fs::write(
        &executable,
        "#!/bin/sh\n[ \"$1\" = '-c' ]\n[ \"$2\" = 'show system' ]\nprintf 'hostname: test\\nuptime: 42\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let uptime = KeeneticLocalAdapter::new(executable).uptime().await.unwrap();
    assert_eq!(uptime, Some(Duration::from_secs(42)));
}

#[tokio::test]
async fn reports_ndmc_failure() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("ndmc");
    fs::write(&executable, "#!/bin/sh\necho rejected >&2\nexit 1\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let error = KeeneticLocalAdapter::new(executable).uptime().await.unwrap_err();
    assert!(error.to_string().contains("rejected"));
}
