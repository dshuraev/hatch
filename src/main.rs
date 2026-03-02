use std::process::ExitCode;

use clap::Parser;
use hatch::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match hatch::app::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
