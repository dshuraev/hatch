use std::process::ExitCode;

use crate::cli::{Cli, Command};
use crate::config::{Config, ConfigError};
use crate::dispatch::{dispatch, DispatchError};

pub fn run(cli: Cli) -> Result<ExitCode, AppError> {
    match cli.command {
        Some(Command::Check { path }) => {
            Config::check_path(&path)?;
            check_config();
            Ok(ExitCode::SUCCESS)
        }
        None => {
            let path = match cli.config {
                Some(path) => path,
                None => Config::default_path()?,
            };
            let config = Config::load_from_path(&path)?;
            run_config(&config)
        }
    }
}

fn run_config(config: &Config) -> Result<ExitCode, AppError> {
    let status = dispatch(config)?;
    Ok(exit_code_from_status(status))
}

fn check_config() {
    // Stub for the future config checker.
}

fn exit_code_from_status(status: std::process::ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) => ExitCode::from(code as u8),
        None => ExitCode::FAILURE,
    }
}

#[derive(Debug)]
pub enum AppError {
    Config(ConfigError),
    Dispatch(DispatchError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Config(error) => error.fmt(f),
            AppError::Dispatch(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Config(error) => Some(error),
            AppError::Dispatch(error) => Some(error),
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
