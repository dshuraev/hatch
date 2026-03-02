use std::path::PathBuf;

use clap::Parser;
use hatch::cli::{Cli, Command};
use hatch::config::Config;

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
    let previous = std::env::var_os("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/hatch-config");
    }

    let default_path = Config::default_path().expect("default path should resolve");

    assert_eq!(default_path, PathBuf::from("/tmp/hatch-config/hatch/hatch.yaml"));

    match previous {
        Some(value) => unsafe {
            std::env::set_var("XDG_CONFIG_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        },
    }
}
