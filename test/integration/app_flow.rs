#![cfg(not(miri))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST_DIR_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn check_succeeds_for_valid_config() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "valid.yaml",
        r#"
commands:
  lock-screen:
    run: printf ok
"#,
    );

    let output = hatch_command()
        .arg("check")
        .arg(&config)
        .output()
        .expect("check command should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "config is valid\n");
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn check_reports_invalid_config() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "invalid.yaml",
        r#"
commands:
  broken: {}
"#,
    );

    let output = hatch_command()
        .arg("check")
        .arg(&config)
        .output()
        .expect("check command should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("must define `run`"));
    assert!(stderr.contains("broken"));
}

#[test]
fn check_reports_missing_config_file() {
    let temp = TestDir::new();
    let missing = temp.path().join("missing.yaml");

    let output = hatch_command()
        .arg("check")
        .arg(&missing)
        .output()
        .expect("check command should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to read config"));
    assert!(stderr.contains("missing.yaml"));
}

#[test]
fn dispatch_returns_executed_command_exit_code() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "dispatch.yaml",
        r#"
commands:
  fail-seven:
    run: exit 7
"#,
    );

    let output = hatch_command()
        .arg("--config")
        .arg(&config)
        .env("SSH_ORIGINAL_COMMAND", "fail-seven")
        .output()
        .expect("dispatch command should run");

    assert_eq!(output.status.code(), Some(7));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn dispatch_uses_default_config_path_from_xdg_config_home() {
    let temp = TestDir::new();
    let config_dir = temp.create_dir("xdg/hatch");
    let config = config_dir.join("hatch.yaml");

    fs::write(
        &config,
        r#"
commands:
  fail-nine:
    run: exit 9
"#,
    )
    .expect("config file should be written");

    let output = hatch_command()
        .env("XDG_CONFIG_HOME", temp.path().join("xdg"))
        .env("SSH_ORIGINAL_COMMAND", "fail-nine")
        .output()
        .expect("dispatch command should run");

    assert_eq!(output.status.code(), Some(9));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn dispatch_hides_config_parse_errors_from_ssh_caller() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "invalid.yaml",
        r#"
commands:
  broken
    run: nope
"#,
    );

    let output = hatch_command()
        .arg("--config")
        .arg(&config)
        .env("SSH_ORIGINAL_COMMAND", "lock-screen")
        .output()
        .expect("dispatch command should run");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "internal error"
    );
}

#[test]
fn dispatch_logs_share_single_dispatch_id() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "dispatch.yaml",
        r#"
commands:
  ok:
    run: exit 0
"#,
    );
    let log_path = temp.path().join("hatch.log");

    let output = hatch_command()
        .arg("--config")
        .arg(&config)
        .env("SSH_ORIGINAL_COMMAND", "ok")
        .env("HATCH_LOG_SINK", "file")
        .env("HATCH_LOG_FILE", &log_path)
        .output()
        .expect("dispatch command should run");

    assert!(output.status.success());

    let logs = fs::read_to_string(&log_path).expect("log file should exist");
    let mut ids = Vec::new();
    for line in logs.lines() {
        let Some(fragment) = line
            .split_whitespace()
            .find(|part| part.starts_with("dispatch_id="))
        else {
            continue;
        };
        let id = fragment.trim_start_matches("dispatch_id=");
        ids.push(id.to_string());
    }

    assert!(ids.len() >= 3, "expected multiple logged events");
    assert!(ids.iter().all(|id| id == &ids[0]));
}

#[test]
#[cfg(unix)]
fn dispatch_applies_absolute_cwd() {
    let temp = TestDir::new();
    let work_dir = temp.create_dir("work");
    let quoted = shell_single_quote(&work_dir.display().to_string());
    let config = temp.write_config(
        "dispatch.yaml",
        &format!(
            r#"
commands:
  check-cwd:
    run: if [ "$PWD" = {quoted} ]; then exit 0; else exit 23; fi
    cwd: {}
"#,
            work_dir.display()
        ),
    );

    let output = hatch_command()
        .arg("--config")
        .arg(&config)
        .env("SSH_ORIGINAL_COMMAND", "check-cwd")
        .output()
        .expect("dispatch command should run");

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg(unix)]
fn dispatch_overlays_env_onto_inherited_environment() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "dispatch.yaml",
        r#"
commands:
  check-env:
    run: if [ "$PARENT_ONLY" = "yes" ] && [ "$OVERRIDE_ME" = "from-config" ] && [ "$NEW_ONLY" = "new" ]; then exit 0; else exit 19; fi
    env:
      OVERRIDE_ME: from-config
      NEW_ONLY: new
"#,
    );

    let output = hatch_command()
        .arg("--config")
        .arg(&config)
        .env("SSH_ORIGINAL_COMMAND", "check-env")
        .env("PARENT_ONLY", "yes")
        .env("OVERRIDE_ME", "from-parent")
        .output()
        .expect("dispatch command should run");

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg(unix)]
fn dispatch_hard_kills_process_on_timeout() {
    let temp = TestDir::new();
    let config = temp.write_config(
        "dispatch.yaml",
        r#"
commands:
  slow:
    run: sleep 2
    timeout: 1
"#,
    );

    let output = hatch_command()
        .arg("--config")
        .arg(&config)
        .env("SSH_ORIGINAL_COMMAND", "slow")
        .output()
        .expect("dispatch command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exceeded timeout"));
}

fn hatch_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hatch"))
}

#[cfg(unix)]
fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let unique = NEXT_TEST_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("hatch-app-flow-{}-{}", std::process::id(), unique));
        fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn write_config(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("config file should be written");
        path
    }

    fn create_dir(&self, path: &str) -> PathBuf {
        let path = self.path.join(path);
        fs::create_dir_all(&path).expect("directory should be created");
        path
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        remove_dir_all_if_exists(&self.path);
    }
}

fn remove_dir_all_if_exists(path: &Path) {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("failed to remove temp dir {}: {error}", path.display()),
    }
}
