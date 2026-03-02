use std::collections::BTreeMap;

use hatch::config::{CommandConfig, Config};
use hatch::dispatch::dispatch;

#[test]
fn dispatch_requires_ssh_original_command() {
    let previous = std::env::var_os("SSH_ORIGINAL_COMMAND");
    unsafe {
        std::env::remove_var("SSH_ORIGINAL_COMMAND");
    }

    let result = dispatch(&sample_config());

    if let Some(value) = previous {
        unsafe {
            std::env::set_var("SSH_ORIGINAL_COMMAND", value);
        }
    }

    assert!(result.is_err());
}

fn sample_config() -> Config {
    let mut commands = BTreeMap::new();
    commands.insert(
        "lock-screen".to_string(),
        CommandConfig::new("printf dispatched"),
    );
    Config::new(commands)
}
