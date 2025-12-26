use routerbot_core::config::Config;
use std::path::PathBuf;

fn write_temp_config(contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time before unix epoch")
        .as_nanos();
    path.push(format!("routerbot-config-{nanos}.toml"));
    std::fs::write(&path, contents).expect("write temp config");
    path
}

#[test]
fn loads_config_from_path() {
    let config_text = r#"
[telegram]
bot_token = "token"

[transmission]
url = "http://localhost:9091"
username = "user"
password = "pass"
"#;
    let path = write_temp_config(config_text);

    let config = Config::load_from_path(&path).expect("load config");

    assert_eq!(config.telegram.bot_token, "token");
    assert_eq!(config.transmission.url, "http://localhost:9091");
    assert_eq!(config.transmission.username.as_deref(), Some("user"));
    assert_eq!(config.transmission.password.as_deref(), Some("pass"));

    std::fs::remove_file(&path).expect("cleanup temp config");
}
