use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub enum Level {
    Info,
    Error,
}

enum Sink {
    Off,
    File(Mutex<std::fs::File>),
    Journald(JournaldSink),
}

pub struct Logger {
    sink: Sink,
}

impl Logger {
    pub fn init_from_env() -> Self {
        let sink = env::var("HATCH_LOG_SINK")
            .unwrap_or_else(|_| "journald".to_string())
            .to_ascii_lowercase();

        match sink.as_str() {
            "off" => Self::off(),
            "file" => {
                let Some(path) = env::var_os("HATCH_LOG_FILE") else {
                    return Self::off();
                };

                match OpenOptions::new().create(true).append(true).open(path) {
                    Ok(file) => Self {
                        sink: Sink::File(Mutex::new(file)),
                    },
                    Err(_) => Self::off(),
                }
            }
            "journald" => JournaldSink::new()
                .map(|journald| Self {
                    sink: Sink::Journald(journald),
                })
                .unwrap_or_else(|_| Self::off()),
            _ => Self::off(),
        }
    }

    pub fn off() -> Self {
        Self { sink: Sink::Off }
    }

    pub fn log<'a, I>(&self, level: Level, event: &str, dispatch_id: &str, fields: I)
    where
        I: IntoIterator<Item = (&'a str, String)>,
    {
        if matches!(self.sink, Sink::Off) {
            return;
        }

        let mut line = format!(
            "ts={} level={} event={} dispatch_id={}",
            unix_seconds(),
            level_name(&level),
            sanitize(event),
            sanitize(dispatch_id)
        );

        for (key, value) in fields {
            line.push(' ');
            line.push_str(&sanitize_key(key));
            line.push('=');
            line.push_str(&sanitize(&value));
        }

        match &self.sink {
            Sink::Off => {}
            Sink::File(file) => {
                if let Ok(mut file) = file.lock() {
                    let _ = writeln!(file, "{line}");
                }
            }
            Sink::Journald(journald) => {
                let _ = journald.send(priority_for_level(&level), event, dispatch_id, &line);
            }
        }
    }
}

pub fn new_dispatch_id() -> String {
    Uuid::new_v4().to_string()
}

fn unix_seconds() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn level_name(level: &Level) -> &'static str {
    match level {
        Level::Info => "info",
        Level::Error => "error",
    }
}

fn priority_for_level(level: &Level) -> u8 {
    match level {
        Level::Info => 6,
        Level::Error => 3,
    }
}

fn sanitize_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize(value: &str) -> String {
    const MAX_LEN: usize = 256;

    let mut out = String::with_capacity(value.len().min(MAX_LEN));
    for ch in value.chars() {
        if out.len() >= MAX_LEN {
            break;
        }

        if ch.is_control() {
            out.push('?');
        } else {
            out.push(ch);
        }
    }

    out
}

struct JournaldSink {
    #[cfg(unix)]
    socket: std::os::unix::net::UnixDatagram,
}

impl JournaldSink {
    fn new() -> Result<Self, std::io::Error> {
        #[cfg(unix)]
        {
            Ok(Self {
                socket: std::os::unix::net::UnixDatagram::unbound()?,
            })
        }

        #[cfg(not(unix))]
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "journald is only available on unix platforms",
            ))
        }
    }

    fn send(
        &self,
        priority: u8,
        event: &str,
        dispatch_id: &str,
        message: &str,
    ) -> Result<(), std::io::Error> {
        #[cfg(unix)]
        {
            let payload = format!(
                "PRIORITY={priority}\nSYSLOG_IDENTIFIER=hatch\nHATCH_EVENT={event}\nHATCH_DISPATCH_ID={dispatch_id}\nMESSAGE={message}\n"
            );
            self.socket
                .send_to(payload.as_bytes(), Path::new("/run/systemd/journal/socket"))?;
            Ok(())
        }

        #[cfg(not(unix))]
        {
            let _ = (priority, event, dispatch_id, message);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "journald is only available on unix platforms",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::new_dispatch_id;

    #[test]
    fn dispatch_id_uses_uuid_shape() {
        let id = new_dispatch_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().nth(8), Some('-'));
        assert_eq!(id.chars().nth(13), Some('-'));
        assert_eq!(id.chars().nth(18), Some('-'));
        assert_eq!(id.chars().nth(23), Some('-'));
    }
}
