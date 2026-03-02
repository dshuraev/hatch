use crate::cli::{Cli, Command};
use crate::config::{Config, ConfigError};

pub fn run(cli: Cli) -> Result<(), ConfigError> {
    match cli.command {
        Some(Command::Check { path }) => {
            let config = Config::load_from_path(&path)?;
            check_config(&config);
            Ok(())
        }
        None => {
            let path = match cli.config {
                Some(path) => path,
                None => Config::default_path()?,
            };
            let config = Config::load_from_path(&path)?;
            run_config(&config);
            Ok(())
        }
    }
}

fn run_config(_config: &Config) {
    // Stub for the future command dispatcher.
}

fn check_config(_config: &Config) {
    // Stub for the future config checker.
}
