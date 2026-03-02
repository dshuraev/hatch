use std::collections::BTreeMap;

use hatch::config::{CommandConfig, Config, ConfigError};

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

#[test]
fn rejects_empty_command_map() {
    let config = Config {
        commands: BTreeMap::new(),
    };

    let error = config.validate().expect_err("config should be invalid");

    assert!(matches!(error, ConfigError::Invalid(_)));
}
