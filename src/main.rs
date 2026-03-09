use std::process::ExitCode;

use clap::Parser;
use hatch::app::RunOutcome;
use hatch::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match hatch::app::run(cli) {
        Ok(RunOutcome::ExitCode(code)) => code,
        Ok(RunOutcome::ProcessExit(code)) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
