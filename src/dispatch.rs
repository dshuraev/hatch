use std::env;
use std::error::Error;
use std::fmt;
use std::process::{Command, ExitStatus};

use crate::config::Config;

pub fn dispatch(config: &Config) -> Result<ExitStatus, DispatchError> {
    dispatch_with(config, env::var("SSH_ORIGINAL_COMMAND").ok(), execute_shell_command)
}

fn dispatch_with<F>(
    config: &Config,
    original_command: Option<String>,
    executor: F,
) -> Result<ExitStatus, DispatchError>
where
    F: FnOnce(&str) -> Result<ExitStatus, std::io::Error>,
{
    let original_command = original_command
        .ok_or(DispatchError::MissingOriginalCommand)?
        .trim()
        .to_string();

    if original_command.is_empty() {
        return Err(DispatchError::MissingOriginalCommand);
    }

    let command = config
        .commands
        .get(&original_command)
        .ok_or_else(|| DispatchError::UnknownCommand(original_command.clone()))?;

    executor(&command.run).map_err(|source| DispatchError::Execute {
        command: original_command,
        source,
    })
}

fn execute_shell_command(command: &str) -> Result<ExitStatus, std::io::Error> {
    Command::new("/bin/sh").arg("-c").arg(command).status()
}

#[derive(Debug)]
pub enum DispatchError {
    MissingOriginalCommand,
    UnknownCommand(String),
    Execute {
        command: String,
        source: std::io::Error,
    },
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchError::MissingOriginalCommand => {
                write!(f, "SSH_ORIGINAL_COMMAND must be set to a configured command key")
            }
            DispatchError::UnknownCommand(command) => {
                write!(f, "command `{command}` is not defined in config")
            }
            DispatchError::Execute { command, source } => {
                write!(f, "failed to execute command `{command}`: {source}")
            }
        }
    }
}

impl Error for DispatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DispatchError::Execute { source, .. } => Some(source),
            DispatchError::MissingOriginalCommand | DispatchError::UnknownCommand(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{dispatch_with, DispatchError};
    use crate::config::{CommandConfig, Config};

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    fn sample_config() -> Config {
        let mut commands = BTreeMap::new();
        commands.insert(
            "lock-screen".to_string(),
            CommandConfig {
                run: "loginctl lock-session".to_string(),
            },
        );
        Config { commands }
    }

    #[test]
    fn rejects_missing_ssh_original_command() {
        let error = dispatch_with(&sample_config(), None, |_| unreachable!())
            .expect_err("dispatch should fail");

        assert!(matches!(error, DispatchError::MissingOriginalCommand));
    }

    #[test]
    fn rejects_unknown_command() {
        let error = dispatch_with(&sample_config(), Some("restart-app".to_string()), |_| {
            unreachable!()
        })
        .expect_err("dispatch should fail");

        assert!(matches!(error, DispatchError::UnknownCommand(command) if command == "restart-app"));
    }

    #[test]
    fn executes_matched_command() {
        let status = dispatch_with(&sample_config(), Some("lock-screen".to_string()), |command| {
            assert_eq!(command, "loginctl lock-session");
            Ok(success_status())
        })
        .expect("dispatch should succeed");

        assert!(status.success());
    }

    #[cfg(unix)]
    fn success_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }
}
