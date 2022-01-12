#![allow(dead_code)] // Fix: remove

mod runtime;
mod shims;
mod tool_versions;

use anyhow::Result;
use clap::Parser;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, LoggerHandle, Naming};
use log::warn;
use runtime::RuntimeEnvironment;

/// General Version Manager for Standalone Command Line Executables
///
/// A (some kind of) clone of `asdf` for windows.
#[derive(Debug, Parser)]
#[clap(name = "asdfw.exe", version)]
struct Cli {
    /// Verbosity level. Specify more than once for more verbosity. By default
    /// only warning and errors are displayed.
    #[structopt(short, parse(from_occurrences), global = true)]
    verbose: usize,

    #[structopt(subcommand)]
    command: CliSubCommand,
}

#[derive(Debug, clap::Subcommand)]
enum CliSubCommand {
    /// Execute a command.
    ///
    /// Execute the configured version of a command. All the args are passed
    /// verbatim to the command. Make sure you prefix all args (after the
    /// command name) with '--'.
    /// Should not be called directly by the user.
    Exec {
        /// The command to execute
        command: String,

        /// all other arguments to pass to the command (add ' -- ' before the
        /// args to avoid option parsing issues)
        args: Vec<String>,
    },
    /// Just a dummy command
    Dummy
}

fn main() -> Result<()> {
    let app = Cli::parse();
    let env = RuntimeEnvironment::new()?;
    let log_level = match app.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    match app.command {
        CliSubCommand::Exec { .. } => log_to_file(&env, &log_level),
        _ => log_to_stderr(&log_level),
    }?;

    warn!("{:?}", app);
    Ok(())
}

fn run(_: Cli) -> Result<()> {
    Ok(())
}

fn log_to_file(env: &RuntimeEnvironment, spec: &str) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str(spec)?
        .log_to_file(FileSpec::default().directory(&env.log_dir))
        .rotate(
            Criterion::Size(1_000_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(4),
        )
        .append()
        .start()?)
}

fn log_to_stderr(spec: &str) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str(spec)?.log_to_stderr().start()?)
}
