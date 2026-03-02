#[path = "../../src/config.rs"]
mod config;

use std::collections::BTreeMap;

use config::{CommandConfig, Config};

#[test]
fn deserializes_readme_config_shape() {
    let yaml = r#"
commands:
  lock-screen:
    run: loginctl lock-session
  restart-app:
    run: systemctl restart app
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("config should deserialize");

    let mut expected = BTreeMap::new();
    expected.insert(
        "lock-screen".to_string(),
        CommandConfig {
            run: "loginctl lock-session".to_string(),
        },
    );
    expected.insert(
        "restart-app".to_string(),
        CommandConfig {
            run: "systemctl restart app".to_string(),
        },
    );

    assert_eq!(config.commands, expected);
}
