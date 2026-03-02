use std::collections::BTreeMap;

use hatch::config::{CommandConfig, Config, ConfigError};
use serde_yaml::Value;

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
    expected.insert("lock-screen".to_string(), CommandConfig::new("loginctl lock-session"));
    expected.insert("restart-app".to_string(), CommandConfig::new("systemctl restart app"));

    assert_eq!(config.commands, expected);
}

#[test]
fn preserves_unknown_top_level_and_command_keys() {
    let yaml = r#"
commands:
  lock-screen:
    run: loginctl lock-session
    timeout: 30
log_level: info
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("config should deserialize");

    assert_eq!(config.extra.get("log_level"), Some(&Value::String("info".to_string())));
    assert_eq!(
        config.commands["lock-screen"].extra.get("timeout"),
        Some(&Value::Number(30.into()))
    );
}

#[test]
fn rejects_invalid_config_with_multiple_diagnostics() {
    let path = write_temp_config(
        "invalid-config.yaml",
        r#"
commands:
  blank-run:
    run: "  "
  missing-run: {}
"#,
    );

    let error = Config::check_path(&path).expect_err("config should be invalid");

    let ConfigError::Invalid(report) = error else {
        panic!("expected invalid config report");
    };

    assert_eq!(report.diagnostics().len(), 2);

    let rendered = report.to_string();
    assert!(rendered.contains("blank-run"));
    assert!(rendered.contains("missing-run"));
    assert!(rendered.contains("run: \"  \""));
    assert!(rendered.contains("missing-run: {}"));
}

#[test]
fn reports_parse_errors_with_location_context() {
    let path = write_temp_config(
        "parse-error.yaml",
        r#"
commands:
  broken
    run: nope
"#,
    );

    let error = Config::check_path(&path).expect_err("config should fail to parse");

    let rendered = error.to_string();
    assert!(rendered.contains("failed to parse YAML"));
    assert!(rendered.contains("parse-error.yaml:"));
    assert!(rendered.contains("broken"));
}

fn write_temp_config(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hatch-{}-{}",
        std::process::id(),
        name
    ));
    std::fs::write(&path, contents).expect("temp config should be writable");
    path
}
