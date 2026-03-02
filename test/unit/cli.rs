use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use clap::Parser;
use hatch::cli::{Cli, Command};
use hatch::config::{Config, ConfigError};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("environment lock should not be poisoned")
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn capture(key: &'static str) -> Self {
        Self {
            key,
            previous: std::env::var_os(key),
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}

#[test]
fn parses_short_config_flag() {
    let cli = Cli::parse_from(["hatch", "-c", "./config.yaml"]);

    assert_eq!(cli.config, Some(PathBuf::from("./config.yaml")));
    assert_eq!(cli.command, None);
}

#[test]
fn parses_check_subcommand() {
    let cli = Cli::parse_from(["hatch", "check", "./config.yaml"]);

    assert_eq!(cli.config, None);
    assert_eq!(
        cli.command,
        Some(Command::Check {
            path: PathBuf::from("./config.yaml"),
        })
    );
}

#[test]
fn resolves_default_path_from_xdg_config_home() {
    let _env_lock = lock_env();
    let _xdg_guard = EnvVarGuard::capture("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/hatch-config");
    }

    let default_path = Config::default_path().expect("default path should resolve");

    assert_eq!(
        default_path,
        PathBuf::from("/tmp/hatch-config/hatch/hatch.yaml")
    );
}

#[test]
fn falls_back_to_home_for_default_path() {
    let _env_lock = lock_env();
    let _xdg_guard = EnvVarGuard::capture("XDG_CONFIG_HOME");
    let _home_guard = EnvVarGuard::capture("HOME");
    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::set_var("HOME", "/tmp/hatch-home");
    }

    let default_path = Config::default_path().expect("default path should resolve");

    assert_eq!(
        default_path,
        PathBuf::from("/tmp/hatch-home/.config/hatch/hatch.yaml")
    );
}

#[test]
fn errors_when_no_config_home_is_available() {
    let _env_lock = lock_env();
    let _xdg_guard = EnvVarGuard::capture("XDG_CONFIG_HOME");
    let _home_guard = EnvVarGuard::capture("HOME");
    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("HOME");
    }

    let error = Config::default_path().expect_err("default path should fail");

    assert!(matches!(error, ConfigError::MissingConfigHome));
}
