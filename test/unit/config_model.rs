use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::PathBuf;

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
    expected.insert(
        "lock-screen".to_string(),
        CommandConfig::new("loginctl lock-session"),
    );
    expected.insert(
        "restart-app".to_string(),
        CommandConfig::new("systemctl restart app"),
    );

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

    assert_eq!(
        config.extra.get("log_level"),
        Some(&Value::String("info".to_string()))
    );
    assert_eq!(config.commands["lock-screen"].timeout, Some(30));
}

#[test]
fn deserializes_execution_controls() {
    let yaml = r#"
commands:
  restart-app:
    run: systemctl restart app
    timeout: 15
    cwd: /opt/app
    env:
      APP_ENV: production
      LOG_LEVEL: warn
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("config should deserialize");
    let command = &config.commands["restart-app"];

    assert_eq!(command.timeout, Some(15));
    assert_eq!(command.cwd, Some(PathBuf::from("/opt/app")));
    assert_eq!(command.env.get("APP_ENV"), Some(&"production".to_string()));
    assert_eq!(command.env.get("LOG_LEVEL"), Some(&"warn".to_string()));
}

#[test]
fn rejects_invalid_config_with_multiple_diagnostics() {
    let error = check_config(
        "invalid-config.yaml",
        r#"
commands:
  blank-run:
    run: "  "
  missing-run: {}
"#,
    )
    .expect_err("config should be invalid");

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
fn rejects_non_positive_timeout() {
    let error = check_config(
        "non-positive-timeout.yaml",
        r#"
commands:
  lock-screen:
    run: loginctl lock-session
    timeout: 0
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("field `timeout` must be greater than 0"));
}

#[test]
fn rejects_relative_cwd() {
    let error = check_config(
        "relative-cwd.yaml",
        r#"
commands:
  lock-screen:
    run: loginctl lock-session
    cwd: ./relative/path
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("field `cwd` must be an absolute path"));
}

#[test]
fn rejects_non_mapping_env() {
    let error = check_config(
        "env-not-mapping.yaml",
        r#"
commands:
  lock-screen:
    run: loginctl lock-session
    env: []
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("field `env` must be a mapping"));
}

#[test]
fn rejects_non_mapping_root() {
    let error = check_config("non-mapping-root.yaml", "- just\n- a\n- list\n")
        .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("config root must be a YAML mapping"));
}

#[test]
fn rejects_missing_commands_mapping() {
    let error = check_config(
        "missing-commands.yaml",
        r#"
log_level: info
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("config must define a top-level `commands` mapping"));
}

#[test]
fn rejects_non_mapping_commands_value() {
    let error = check_config(
        "commands-not-mapping.yaml",
        r#"
commands: []
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("top-level `commands` value must be a mapping"));
}

#[test]
fn rejects_empty_commands_mapping() {
    let error = check_config(
        "empty-commands.yaml",
        r#"
commands: {}
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("config must define at least one command"));
}

#[test]
fn rejects_non_mapping_command_entry() {
    let error = check_config(
        "command-not-mapping.yaml",
        r#"
commands:
  lock-screen: loginctl lock-session
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("command `lock-screen` must be a mapping"));
}

#[test]
fn rejects_non_string_run_value() {
    let error = check_config(
        "non-string-run.yaml",
        r#"
commands:
  lock-screen:
    run: 42
"#,
    )
    .expect_err("config should be invalid");

    let rendered = error.to_string();
    assert!(rendered.contains("command `lock-screen` field `run` must be a string"));
}

#[test]
fn reports_parse_errors_with_location_context() {
    let error = check_config(
        "parse-error.yaml",
        r#"
commands:
  broken
    run: nope
"#,
    )
    .expect_err("config should fail to parse");

    let rendered = error.to_string();
    assert!(rendered.contains("failed to parse YAML"));
    assert!(rendered.contains("parse-error.yaml:"));
    assert!(rendered.contains("broken"));
}

fn check_config(name: &str, contents: &str) -> Result<(), ConfigError> {
    Config::check_reader(PathBuf::from(name), Cursor::new(contents.as_bytes()))
}
