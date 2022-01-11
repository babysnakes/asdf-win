#![allow(dead_code)] // Fix: remove

mod runtime;
mod shims;
mod tool_versions;

use anyhow::Result;
use clap::Parser;

/// General Version Manager for Standalone Command Line Executables
///
/// A (some kind of) clone of `asdf` for windows.
#[derive(Debug, Parser)]
#[clap(name = "asdfw.exe", version)]
struct Cli {
    /// Activate debug mode (specify more then once for more verbosity)
    #[structopt(short, parse(from_occurrences), global=true)]
    debug: usize,

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
    }
}

fn main() {
    let app = Cli::parse();
    println!("{:?}", app);
}

fn run(_: Cli) -> Result<()> {
    Ok(())
}