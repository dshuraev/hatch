use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(name = "hatch")]
pub struct Cli {
    #[arg(short = 'c', long = "config")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum Command {
    Check { path: PathBuf },
    List,
}
