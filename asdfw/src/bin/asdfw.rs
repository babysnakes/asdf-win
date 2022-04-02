use anyhow::{anyhow, Result};
use asdfw::common::*;
use asdfw::plugins::plugin_manager::PluginManager;
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
    /// Get full path to configured version for command
    Which {
        /// The command to get the full path for (could omit extension)
        cmd: String,
    },
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
    log_to_file(&env, log_level)?;

    run(app, &env)
}

fn run(app: Cli, env: &RuntimeEnvironment) -> Result<()> {
    match app.command {
        CliSubCommand::Reshim { cleanup } => reshim(env, cleanup),
        CliSubCommand::Local { tool, version } => set_local(env, &tool, &version),
        CliSubCommand::Global { tool, version } => set_global(env, &tool, &version),
        CliSubCommand::Completion => gen_completions(),
        CliSubCommand::Which { cmd } => which(env, &cmd),
    }
}

fn reshim(env: &RuntimeEnvironment, cleanup: bool) -> Result<()> {
    info!("Create shims requested");
    let pm = PluginManager::new(&env.plugins_dir);
    let shims = Shims::new(&env.shims_db, &env.installs_dir, &env.shims_dir, &env.shim_exe, &pm)?;
    let db = shims.generate_db_from_installed_tools()?;
    shims.save_db(&db)?;
    shims.create_shims(cleanup)?;
    let output = success_message("Reshim finished successfully.");
    print_out(output);
    Ok(())
}

fn set_global<'a>(env: &RuntimeEnvironment, tool: &'a str, version: &'a str) -> Result<()> {
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, tool);
    tvs.save_global(version)?;
    let msg = format!("Successfully configured global version ({}) for {}", &version, &tool);
    let output = success_message(&msg);
    print_out(output);
    Ok(())
}

fn gen_completions() -> Result<()> {
    let mut app = Cli::into_app();
    generate(PowerShell, &mut app, APP_NAME, &mut std::io::stdout());
    Ok(())
}

fn set_local<'a>(env: &RuntimeEnvironment, tool: &'a str, version: &'a str) -> Result<()> {
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, tool);
    tvs.save_local(version)?;
    let msg = format!("Successfully configured local version ({}) for {}", &version, &tool);
    let output = success_message(&msg);
    print_out(output);
    Ok(())
}

fn which(env: &RuntimeEnvironment, cmd: &str) -> Result<()> {
    info!("invoked `which` on {}", &cmd);
    let path = find_path_for_cmd(env, cmd)?;
    let path_name = path
        .to_str()
        .ok_or_else(|| anyhow!("Could not convert tool name {:?} to UTF-8. Are you using non UTF-8 charset?", &path))?;
    print_out(vec![path_name]);
    Ok(())
}

fn log_to_file(env: &RuntimeEnvironment, spec: &str) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str(spec)?
        .log_to_file(FileSpec::default().directory(&env.log_dir))
        .rotate(Criterion::Size(1_000_000), Naming::Numbers, Cleanup::KeepLogFiles(4))
        .append()
        .start()?)
}
