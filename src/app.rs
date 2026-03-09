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
        Some(Command::List) => {
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
                    ("mode", "list".to_string()),
                    ("config_path_hint", config_hint),
                ],
            );

            let config = load_runtime_config(cli.config, &logger, &dispatch_id)?;
            list_commands(&config);
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

            let config = load_runtime_config(cli.config, &logger, &dispatch_id)?;
            run_config(&config, &logger, &dispatch_id)
        }
    }
}

fn load_runtime_config(
    config_path: Option<std::path::PathBuf>,
    logger: &Logger,
    dispatch_id: &str,
) -> Result<Config, AppError> {
    let path = match config_path {
        Some(path) => path,
        None => match Config::default_path() {
            Ok(path) => path,
            Err(error) => {
                logger.log(
                    Level::Error,
                    "config_path_resolution_failed",
                    dispatch_id,
                    vec![("error", error.to_string())],
                );
                return Err(AppError::Internal);
            }
        },
    };
    logger.log(
        Level::Info,
        "config_path_resolved",
        dispatch_id,
        vec![("config_path", path.display().to_string())],
    );

    Config::load_from_path(&path).map_err(|error| {
        logger.log(
            Level::Error,
            "config_load_failed",
            dispatch_id,
            vec![
                ("config_path", path.display().to_string()),
                ("error", error.to_string()),
            ],
        );
        AppError::Internal
    })
}

fn run_config(config: &Config, logger: &Logger, dispatch_id: &str) -> Result<RunOutcome, AppError> {
    let status = dispatch(config, logger, dispatch_id)?;
    Ok(outcome_from_status(status))
}

fn check_config() {
    println!("config is valid");
}

fn list_commands(config: &Config) {
    for command in config.commands.keys() {
        println!("{command}");
    }
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
