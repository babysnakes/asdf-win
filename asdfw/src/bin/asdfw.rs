#![allow(dead_code)] // Fix: remove

use anyhow::Result;
use asdfw::output::*;
use asdfw::runtime::RuntimeEnvironment;
use asdfw::shims::Shims;
use clap::Parser;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, LoggerHandle, Naming};

/// General Version Manager for Standalone Command Line Executables
///
/// A (some kind of) clone of `asdf` for windows.
#[derive(Debug, Parser)]
#[clap(name = "asdfw.exe", version)]
struct Cli {
    /// Verbosity level. Specify more than once for more verbosity. By default
    /// only warning and errors are displayed.
    #[clap(short, parse(from_occurrences), global = true)]
    verbose: usize,

    #[clap(subcommand)]
    command: CliSubCommand,
}

#[derive(Debug, clap::Subcommand)]
enum CliSubCommand {
    /// Recreate shims.
    ///
    /// Recreate the shims.db and the shims (currently not working)
    Reshim,
}

fn main() {
    let app = Cli::parse();

    match do_main(app) {
        Ok(_) => {}
        Err(err) => {
            let output = output_full_error(err, None);
            print_out(output);
            std::process::exit(1);
        }
    }
}

fn do_main(app: Cli) -> Result<()> {
    let env = RuntimeEnvironment::new()?;
    let log_level = match app.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    log_to_file(&env, &log_level)?;

    run(app, &env)
}

fn run(app: Cli, env: &RuntimeEnvironment) -> Result<()> {
    match app.command {
        CliSubCommand::Reshim => reshim(&env),
    }
}

fn reshim(env: &RuntimeEnvironment) -> Result<()> {
    let shims = Shims::new(&env.shims_db, &env.installs_dir)?;
    let db = shims.generate_db_from_installed_tools()?;
    shims.save_db(&db)?;
    let output = success_message("Reshim finished successfully.");
    Ok(print_out(output))
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
