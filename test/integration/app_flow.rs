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

fn hatch_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hatch"))
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let unique = NEXT_TEST_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("hatch-app-flow-{}-{}", std::process::id(), unique));
        fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn write_config(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("config file should be written");
        path
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
