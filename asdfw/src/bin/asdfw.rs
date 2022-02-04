use anyhow::Result;
use asdfw::runtime::RuntimeEnvironment;
use asdfw::shims::Shims;
use asdfw::{output::*, tool_versions::ToolVersions};
use clap::{IntoApp, Parser};
use clap_complete::{generate, shells::PowerShell};
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, LoggerHandle, Naming};
use log::info;

const APP_NAME: &str = "asdfw";

/// General Version Manager for Standalone Command Line Executables
///
/// A (some kind of) clone of `asdf` for windows.
#[derive(Debug, Parser)]
#[clap(name = APP_NAME, version)]
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
    Reshim {
        /// Cleanup all existing shims before creating them. This will remove
        /// dangling shims.
        #[clap(long)]
        cleanup: bool,
    },
    /// Configure tool's version in current directory.
    Local {
        /// The tool to configure the version for
        tool: String,
        /// The version to use locally for the specified tool
        version: String,
    },
    /// Configure tool's version globally.
    Global {
        /// The tool to configure the version for
        tool: String,
        /// The version to use globally for the specified tool
        version: String,
    },
    /// Generate completion.
    ///
    /// The output of this command could be redirected to a file to be loaded at
    /// shell initialization for invoked directly:
    ///
    ///     asdfw.exe completions | Out-String | Invoke-Expression
    Completion,
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
        CliSubCommand::Reshim { cleanup } => reshim(&env, cleanup),
        CliSubCommand::Local { tool, version } => set_local(env, &tool, &version),
        CliSubCommand::Global { tool, version } => set_global(env, &tool, &version),
        CliSubCommand::Completion => gen_completions(),
    }
}

fn reshim(env: &RuntimeEnvironment, cleanup: bool) -> Result<()> {
    info!("Create shims requested");
    let shims = Shims::new(
        &env.shims_db,
        &env.installs_dir,
        &env.shims_dir,
        &env.shim_exe,
    )?;
    let db = shims.generate_db_from_installed_tools()?;
    shims.save_db(&db)?;
    shims.create_shims(cleanup)?;
    let output = success_message("Reshim finished successfully.");
    Ok(print_out(output))
}

fn set_global<'a>(env: &RuntimeEnvironment, tool: &'a str, version: &'a str) -> Result<()> {
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, &tool);
    tvs.save_global(&version)?;
    let msg = format!(
        "Successfully configured global version ({}) for {}",
        &version, &tool
    );
    let output = success_message(&msg);
    Ok(print_out(output))
}

fn gen_completions<'a>() -> Result<()> {
    let mut app = Cli::into_app();
    generate(PowerShell, &mut app, APP_NAME, &mut std::io::stdout());
    Ok(())
}

fn set_local<'a>(env: &RuntimeEnvironment, tool: &'a str, version: &'a str) -> Result<()> {
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, &tool);
    tvs.save_local(&version)?;
    let msg = format!(
        "Successfully configured local version ({}) for {}",
        &version, &tool
    );
    let output = success_message(&msg);
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
