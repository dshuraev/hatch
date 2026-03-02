use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub commands: BTreeMap<String, CommandConfig>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandConfig {
    pub run: String,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

impl Config {
    pub fn new(commands: BTreeMap<String, CommandConfig>) -> Self {
        Self {
            commands,
            ..Self::default()
        }
    }

    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        ConfigDocument::load(path)?.decode()
    }

    pub fn check_path(path: &Path) -> Result<(), ConfigError> {
        ConfigDocument::load(path)?.validate()
    }

    pub fn default_path() -> Result<PathBuf, ConfigError> {
        if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(path).join("hatch").join("hatch.yaml"));
        }

        let home = env::var_os("HOME").ok_or(ConfigError::MissingConfigHome)?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("hatch")
            .join("hatch.yaml"))
    }
}

impl CommandConfig {
    pub fn new(run: impl Into<String>) -> Self {
        Self {
            run: run.into(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
struct ConfigDocument {
    path: PathBuf,
    source: String,
    value: Value,
}

impl ConfigDocument {
    fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(path, source)
    }

    fn parse(path: &Path, source: String) -> Result<Self, ConfigError> {
        let value = serde_yaml::from_str::<Value>(&source).map_err(|source_error| {
            let location = source_error.location().map(DiagnosticLocation::from);
            let diagnostic = ConfigDiagnostic {
                message: format!("failed to parse YAML: {source_error}"),
                location,
            };
            ConfigError::Invalid(ConfigReport::new(path.to_path_buf(), source.clone(), vec![diagnostic]))
        })?;

        Ok(Self {
            path: path.to_path_buf(),
            source,
            value,
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let diagnostics = validate_root(&self.value, &self.source);
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Invalid(ConfigReport::new(
                self.path.clone(),
                self.source.clone(),
                diagnostics,
            )))
        }
    }

    fn decode(self) -> Result<Config, ConfigError> {
        self.validate()?;
        serde_yaml::from_value(self.value).map_err(|source| {
            let diagnostic = ConfigDiagnostic {
                message: format!("failed to decode config: {source}"),
                location: None,
            };
            ConfigError::Invalid(ConfigReport::new(self.path, self.source, vec![diagnostic]))
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConfigReport {
    path: PathBuf,
    source: String,
    diagnostics: Vec<ConfigDiagnostic>,
}

impl ConfigReport {
    fn new(path: PathBuf, source: String, diagnostics: Vec<ConfigDiagnostic>) -> Self {
        Self {
            path,
            source,
            diagnostics,
        }
    }

    pub fn diagnostics(&self) -> &[ConfigDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for ConfigReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", diagnostic.render(&self.path, &self.source))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub message: String,
    pub location: Option<DiagnosticLocation>,
}

impl ConfigDiagnostic {
    fn render(&self, path: &Path, source: &str) -> String {
        match self.location {
            Some(location) => render_located_diagnostic(path, source, location, &self.message),
            None => format!("{}: {}", path.display(), self.message),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticLocation {
    pub line: usize,
    pub column: usize,
}

impl From<serde_yaml::Location> for DiagnosticLocation {
    fn from(value: serde_yaml::Location) -> Self {
        Self {
            line: value.line(),
            column: value.column(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    MissingConfigHome,
    Read { path: PathBuf, source: std::io::Error },
    Invalid(ConfigReport),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingConfigHome => {
                write!(f, "unable to resolve config directory from XDG_CONFIG_HOME or HOME")
            }
            ConfigError::Read { path, source } => {
                write!(f, "failed to read config from {}: {source}", path.display())
            }
            ConfigError::Invalid(report) => report.fmt(f),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::MissingConfigHome | ConfigError::Invalid(_) => None,
        }
    }
}

fn validate_root(value: &Value, source: &str) -> Vec<ConfigDiagnostic> {
    let mut diagnostics = Vec::new();

    let root = match value.as_mapping() {
        Some(mapping) => mapping,
        None => {
            diagnostics.push(ConfigDiagnostic {
                message: "config root must be a YAML mapping".to_string(),
                location: Some(DiagnosticLocation { line: 1, column: 1 }),
            });
            return diagnostics;
        }
    };

    let commands_value = match mapping_get(root, "commands") {
        Some(value) => value,
        None => {
            diagnostics.push(ConfigDiagnostic {
                message: "config must define a top-level `commands` mapping".to_string(),
                location: locate_key(source, &["commands"]).or(Some(DiagnosticLocation {
                    line: 1,
                    column: 1,
                })),
            });
            return diagnostics;
        }
    };

    let commands = match commands_value.as_mapping() {
        Some(mapping) => mapping,
        None => {
            diagnostics.push(ConfigDiagnostic {
                message: "top-level `commands` value must be a mapping".to_string(),
                location: locate_key(source, &["commands"]),
            });
            return diagnostics;
        }
    };

    if commands.is_empty() {
        diagnostics.push(ConfigDiagnostic {
            message: "config must define at least one command".to_string(),
            location: locate_key(source, &["commands"]),
        });
        return diagnostics;
    }

    for (key, value) in commands {
        validate_command_entry(source, key, value, &mut diagnostics);
    }

    diagnostics
}

fn validate_command_entry(
    source: &str,
    key: &Value,
    value: &Value,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) {
    let Some(command_name) = key.as_str() else {
        diagnostics.push(ConfigDiagnostic {
            message: "command names must be YAML strings".to_string(),
            location: None,
        });
        return;
    };

    let command_key_path = ["commands", command_name];

    if command_name.trim().is_empty() {
        diagnostics.push(ConfigDiagnostic {
            message: "command names must not be blank".to_string(),
            location: locate_key(source, &command_key_path),
        });
    }

    let command_mapping = match value.as_mapping() {
        Some(mapping) => mapping,
        None => {
            diagnostics.push(ConfigDiagnostic {
                message: format!("command `{command_name}` must be a mapping"),
                location: locate_key(source, &command_key_path),
            });
            return;
        }
    };

    let run_value = match mapping_get(command_mapping, "run") {
        Some(value) => value,
        None => {
            diagnostics.push(ConfigDiagnostic {
                message: format!("command `{command_name}` must define `run`"),
                location: locate_key(source, &command_key_path),
            });
            return;
        }
    };

    match run_value.as_str() {
        Some(run) if !run.trim().is_empty() => {}
        Some(_) => diagnostics.push(ConfigDiagnostic {
            message: format!("command `{command_name}` must define a non-empty `run` value"),
            location: locate_key(source, &["commands", command_name, "run"]),
        }),
        None => diagnostics.push(ConfigDiagnostic {
            message: format!("command `{command_name}` field `run` must be a string"),
            location: locate_key(source, &["commands", command_name, "run"]),
        }),
    }
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn locate_key(source: &str, path: &[&str]) -> Option<DiagnosticLocation> {
    if path.is_empty() {
        return None;
    }

    let mut stack: Vec<(usize, String)> = Vec::new();

    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }

        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        let Some((raw_key, _)) = trimmed.split_once(':') else {
            continue;
        };
        let key = normalize_yaml_key(raw_key);

        while let Some((stack_indent, _)) = stack.last() {
            if *stack_indent >= indent {
                stack.pop();
            } else {
                break;
            }
        }

        stack.push((indent, key.clone()));

        if stack.len() == path.len() && stack.iter().map(|(_, key)| key.as_str()).eq(path.iter().copied()) {
            let column = indent + 1;
            return Some(DiagnosticLocation {
                line: line_index + 1,
                column,
            });
        }
    }

    None
}

fn normalize_yaml_key(raw_key: &str) -> String {
    let trimmed = raw_key.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.as_bytes()[0];
        let last = trimmed.as_bytes()[trimmed.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn render_located_diagnostic(
    path: &Path,
    source: &str,
    location: DiagnosticLocation,
    message: &str,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let line_index = location.line.saturating_sub(1);

    if line_index >= lines.len() {
        return format!(
            "{}:{}:{}: {}",
            path.display(),
            location.line,
            location.column,
            message
        );
    }

    let start = line_index.saturating_sub(1);
    let end = (line_index + 1).min(lines.len().saturating_sub(1));
    let width = (end + 1).to_string().len();

    let mut rendered = format!(
        "{}:{}:{}: {}",
        path.display(),
        location.line,
        location.column,
        message
    );

    for (current, line) in lines.iter().enumerate().take(end + 1).skip(start) {
        rendered.push('\n');
        rendered.push_str(&format!(
            "{:>width$} | {}",
            current + 1,
            line,
            width = width
        ));

        if current == line_index {
            rendered.push('\n');
            rendered.push_str(&format!(
                "{:>width$} | {}^",
                "",
                " ".repeat(location.column.saturating_sub(1)),
                width = width
            ));
        }
    }

    rendered
}
