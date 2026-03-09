use std::process::ExitCode;

use crate::cli::{Cli, Command};
use crate::config::{Config, ConfigError};
use crate::dispatch::{DispatchError, dispatch};
use crate::logging::{Level, Logger, new_dispatch_id};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunOutcome {
    ExitCode(ExitCode),
    ProcessExit(i32),
}

pub fn run(cli: Cli) -> Result<RunOutcome, AppError> {
    let logger = Logger::init_from_env();
    let dispatch_id = new_dispatch_id();

    match cli.command {
        Some(Command::Check { path }) => {
            logger.log(
                Level::Info,
                "startup",
                &dispatch_id,
                vec![
                    ("mode", "check".to_string()),
                    ("config_path", path.display().to_string()),
                ],
            );
            Config::check_path(&path)?;
            check_config();
            Ok(RunOutcome::ExitCode(ExitCode::SUCCESS))
        }
        None => {
            let config_hint = cli
                .config
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<default>".to_string());
            logger.log(
                Level::Info,
                "startup",
                &dispatch_id,
                vec![
                    ("mode", "dispatch".to_string()),
                    ("config_path_hint", config_hint),
                ],
            );

            let path = match cli.config {
                Some(path) => path,
                None => match Config::default_path() {
                    Ok(path) => path,
                    Err(error) => {
                        logger.log(
                            Level::Error,
                            "config_path_resolution_failed",
                            &dispatch_id,
                            vec![("error", error.to_string())],
                        );
                        return Err(AppError::Internal);
                    }
                },
            };
            logger.log(
                Level::Info,
                "config_path_resolved",
                &dispatch_id,
                vec![("config_path", path.display().to_string())],
            );

            let config = match Config::load_from_path(&path) {
                Ok(config) => config,
                Err(error) => {
                    logger.log(
                        Level::Error,
                        "config_load_failed",
                        &dispatch_id,
                        vec![
                            ("config_path", path.display().to_string()),
                            ("error", error.to_string()),
                        ],
                    );
                    return Err(AppError::Internal);
                }
            };
            run_config(&config, &logger, &dispatch_id)
        }
    }
}

fn run_config(config: &Config, logger: &Logger, dispatch_id: &str) -> Result<RunOutcome, AppError> {
    let status = dispatch(config, logger, dispatch_id)?;
    Ok(outcome_from_status(status))
}

fn check_config() {
    println!("config is valid");
}

fn outcome_from_status(status: std::process::ExitStatus) -> RunOutcome {
    match status.code() {
        Some(code) => RunOutcome::ProcessExit(code),
        None => RunOutcome::ExitCode(ExitCode::FAILURE),
    }
}

#[derive(Debug)]
pub enum AppError {
    Config(ConfigError),
    Dispatch(DispatchError),
    Internal,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Config(error) => error.fmt(f),
            AppError::Dispatch(error) => error.fmt(f),
            AppError::Internal => write!(f, "internal error"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Config(error) => Some(error),
            AppError::Dispatch(error) => Some(error),
            AppError::Internal => None,
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(value: ConfigError) -> Self {
        AppError::Config(value)
    }
}

impl From<DispatchError> for AppError {
    fn from(value: DispatchError) -> Self {
        AppError::Dispatch(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{RunOutcome, outcome_from_status};
    use std::process::ExitCode;

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    #[test]
    #[cfg(unix)]
    fn returns_process_exit_code_when_available() {
        let status = std::process::ExitStatus::from_raw(7 << 8);

        assert_eq!(outcome_from_status(status), RunOutcome::ProcessExit(7));
    }

    #[test]
    #[cfg(unix)]
    fn returns_failure_when_process_has_no_exit_code() {
        let status = std::process::ExitStatus::from_raw(9);

        assert_eq!(
            outcome_from_status(status),
            RunOutcome::ExitCode(ExitCode::FAILURE)
        );
    }

    #[test]
    #[cfg(windows)]
    fn preserves_large_windows_exit_codes() {
        let status = std::process::ExitStatus::from_raw(259);

        assert_eq!(outcome_from_status(status), RunOutcome::ProcessExit(259));
    }
}
