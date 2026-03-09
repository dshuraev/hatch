use std::env;
use std::error::Error;
use std::fmt;
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::{CommandConfig, Config};
use crate::logging::{Level, Logger};

pub fn dispatch(
    config: &Config,
    logger: &Logger,
    dispatch_id: &str,
) -> Result<ExitStatus, DispatchError> {
    dispatch_with(
        config,
        env::var("SSH_ORIGINAL_COMMAND").ok(),
        execute_shell_command,
        |level, event, fields| logger.log(level, event, dispatch_id, fields),
    )
}

fn dispatch_with<F, L>(
    config: &Config,
    original_command: Option<String>,
    executor: F,
    mut log: L,
) -> Result<ExitStatus, DispatchError>
where
    F: FnOnce(&CommandConfig) -> Result<ExitStatus, ExecuteError>,
    L: FnMut(Level, &str, Vec<(&'static str, String)>),
{
    log(Level::Info, "dispatch_start", vec![]);

    let original_command = original_command
        .ok_or_else(|| {
            log(
                Level::Error,
                "dispatch_error",
                vec![("error_kind", "missing_original_command".to_string())],
            );
            DispatchError::MissingOriginalCommand
        })?
        .trim()
        .to_string();

    if original_command.is_empty() {
        log(
            Level::Error,
            "dispatch_error",
            vec![("error_kind", "missing_original_command".to_string())],
        );
        return Err(DispatchError::MissingOriginalCommand);
    }

    log(
        Level::Info,
        "dispatch_match_attempt",
        vec![("ssh_original_command", original_command.clone())],
    );

    let command = config.commands.get(&original_command).ok_or_else(|| {
        log(
            Level::Error,
            "dispatch_error",
            vec![
                ("error_kind", "unknown_command".to_string()),
                ("ssh_original_command", original_command.clone()),
            ],
        );
        DispatchError::UnknownCommand(original_command.clone())
    })?;

    log(
        Level::Info,
        "dispatch_match",
        vec![("ssh_original_command", original_command.clone())],
    );

    log(
        Level::Info,
        "dispatch_exec",
        dispatch_fields(&original_command, command),
    );

    match executor(command) {
        Ok(status) => {
            log(
                Level::Info,
                "dispatch_result",
                vec![
                    ("ssh_original_command", original_command),
                    ("exit_code", status.code().unwrap_or(-1).to_string()),
                    ("success", status.success().to_string()),
                ],
            );
            Ok(status)
        }
        Err(ExecuteError::Io(source)) => {
            log(
                Level::Error,
                "dispatch_error",
                vec![
                    ("error_kind", "execute_failed".to_string()),
                    ("ssh_original_command", original_command.clone()),
                    ("io_error", source.to_string()),
                ],
            );
            Err(DispatchError::Execute {
                command: original_command,
                source,
            })
        }
        Err(ExecuteError::Timeout { timeout_secs }) => {
            log(
                Level::Error,
                "dispatch_error",
                vec![
                    ("error_kind", "execute_timeout".to_string()),
                    ("ssh_original_command", original_command.clone()),
                    ("timeout_secs", timeout_secs.to_string()),
                ],
            );
            Err(DispatchError::Timeout {
                command: original_command,
                timeout_secs,
            })
        }
    }
}

fn dispatch_fields(original_command: &str, command: &CommandConfig) -> Vec<(&'static str, String)> {
    let mut fields = vec![
        ("ssh_original_command", original_command.to_string()),
        ("run", command.run.clone()),
    ];

    if let Some(timeout) = command.timeout {
        fields.push(("timeout_secs", timeout.to_string()));
    }

    if let Some(cwd) = &command.cwd {
        fields.push(("cwd", cwd.display().to_string()));
    }

    if !command.env.is_empty() {
        fields.push(("env_keys", command.env.len().to_string()));
    }

    fields
}

fn execute_shell_command(command: &CommandConfig) -> Result<ExitStatus, ExecuteError> {
    let mut process = shell_command(command);
    let mut child = process.spawn().map_err(ExecuteError::Io)?;

    match command.timeout {
        Some(timeout_secs) => wait_with_timeout(&mut child, timeout_secs),
        None => child.wait().map_err(ExecuteError::Io),
    }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout_secs: u64,
) -> Result<ExitStatus, ExecuteError> {
    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();

    loop {
        if let Some(status) = child.try_wait().map_err(ExecuteError::Io)? {
            return Ok(status);
        }

        if start.elapsed() >= timeout {
            match child.kill() {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {}
                Err(error) => return Err(ExecuteError::Io(error)),
            }
            let _ = child.wait();
            return Err(ExecuteError::Timeout { timeout_secs });
        }

        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn shell_command(command: &CommandConfig) -> Command {
    let mut process = Command::new("/bin/sh");
    process.arg("-c").arg(&command.run);
    if let Some(cwd) = &command.cwd {
        process.current_dir(cwd);
    }
    if !command.env.is_empty() {
        process.envs(&command.env);
    }
    process
}

#[cfg(windows)]
fn shell_command(command: &CommandConfig) -> Command {
    let mut process = Command::new("cmd.exe");
    process.arg("/C").arg(&command.run);
    if let Some(cwd) = &command.cwd {
        process.current_dir(cwd);
    }
    if !command.env.is_empty() {
        process.envs(&command.env);
    }
    process
}

#[derive(Debug)]
enum ExecuteError {
    Io(std::io::Error),
    Timeout { timeout_secs: u64 },
}

#[derive(Debug)]
pub enum DispatchError {
    MissingOriginalCommand,
    UnknownCommand(String),
    Timeout {
        command: String,
        timeout_secs: u64,
    },
    Execute {
        command: String,
        source: std::io::Error,
    },
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchError::MissingOriginalCommand => {
                write!(
                    f,
                    "SSH_ORIGINAL_COMMAND must be set to a configured command key"
                )
            }
            DispatchError::UnknownCommand(command) => {
                write!(f, "command `{command}` is not defined in config")
            }
            DispatchError::Timeout {
                command,
                timeout_secs,
            } => {
                write!(
                    f,
                    "command `{command}` exceeded timeout after {timeout_secs}s and was killed"
                )
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
            DispatchError::MissingOriginalCommand
            | DispatchError::UnknownCommand(_)
            | DispatchError::Timeout { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io;

    use super::{DispatchError, ExecuteError, dispatch_with};
    use crate::config::{CommandConfig, Config};

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    fn sample_config() -> Config {
        let mut commands = BTreeMap::new();
        commands.insert(
            "lock-screen".to_string(),
            CommandConfig::new("loginctl lock-session"),
        );
        Config::new(commands)
    }

    #[test]
    fn rejects_missing_ssh_original_command() {
        let error = dispatch_with(&sample_config(), None, |_| unreachable!(), |_, _, _| {})
            .expect_err("dispatch should fail");

        assert!(matches!(error, DispatchError::MissingOriginalCommand));
    }

    #[test]
    fn rejects_unknown_command() {
        let error = dispatch_with(
            &sample_config(),
            Some("restart-app".to_string()),
            |_| unreachable!(),
            |_, _, _| {},
        )
        .expect_err("dispatch should fail");

        assert!(
            matches!(error, DispatchError::UnknownCommand(command) if command == "restart-app")
        );
    }

    #[test]
    fn rejects_blank_ssh_original_command() {
        let error = dispatch_with(
            &sample_config(),
            Some("   ".to_string()),
            |_| unreachable!(),
            |_, _, _| {},
        )
        .expect_err("dispatch should fail");

        assert!(matches!(error, DispatchError::MissingOriginalCommand));
    }

    #[test]
    fn surfaces_executor_failures() {
        let error = dispatch_with(
            &sample_config(),
            Some("lock-screen".to_string()),
            |_| {
                Err(ExecuteError::Io(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "permission denied",
                )))
            },
            |_, _, _| {},
        )
        .expect_err("dispatch should fail");

        assert!(matches!(
            error,
            DispatchError::Execute { command, source }
                if command == "lock-screen"
                && source.kind() == io::ErrorKind::PermissionDenied
        ));
    }

    #[test]
    fn surfaces_executor_timeouts() {
        let error = dispatch_with(
            &sample_config(),
            Some("lock-screen".to_string()),
            |_| Err(ExecuteError::Timeout { timeout_secs: 3 }),
            |_, _, _| {},
        )
        .expect_err("dispatch should fail");

        assert!(matches!(
            error,
            DispatchError::Timeout {
                command,
                timeout_secs
            } if command == "lock-screen" && timeout_secs == 3
        ));
    }

    #[test]
    fn executes_matched_command() {
        let status = dispatch_with(
            &sample_config(),
            Some("lock-screen".to_string()),
            |command| {
                assert_eq!(command.run, "loginctl lock-session");
                Ok(success_status())
            },
            |_, _, _| {},
        )
        .expect("dispatch should succeed");

        assert!(status.success());
    }

    #[cfg(unix)]
    fn success_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }
}
