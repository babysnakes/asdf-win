use anyhow::{anyhow, Context, Result};
use asdfw::shims::Shims;
use asdfw::subcommand::{exec, mk_command};
use asdfw::tool_versions::ToolVersions;
use asdfw::{plugins::plugin_manager::PluginManager, runtime::RuntimeEnvironment};
use flexi_logger::*;
use std::{env, process};

const ERROR_PREFIX: &str = "ASDFW ERROR";
const DEBUG_VARIABLE: &str = "ASDFW_DEBUG_SHIM";

fn main() -> Result<()> {
    match run() {
        Ok(0) => Ok(()),
        Ok(exit_code) => process::exit(exit_code),
        Err(err) => Err(err).context(ERROR_PREFIX),
    }
}

fn run() -> Result<i32> {
    let me = env::current_exe()?;
    let exe_name = me.file_name().unwrap().to_str().unwrap();
    let args = env::args().skip(1);
    let runtime = RuntimeEnvironment::new()?;
    if env::var(DEBUG_VARIABLE).is_ok() {
        configure_log(&runtime)?;
    };
    let pm = PluginManager::new(&runtime.plugins_dir);
    let shims = Shims::new(&runtime.shims_db, &runtime.installs_dir, &runtime.shims_dir, &runtime.shim_exe, pm)?;
    let tool = shims
        .find_plugin(exe_name)?
        .ok_or_else(|| anyhow!("No tool configured for the command: {}", &exe_name))?;
    let tool_versions = ToolVersions::new(&runtime.global_tool_versions_file, &runtime.current_dir, &tool);
    match tool_versions.get_version()? {
        Some(version) => match shims.get_full_executable_path(exe_name, &tool, &version)? {
            Some(cmd) => exec(mk_command(&cmd, args)),
            None => Err(anyhow!("Version '{}' of '{}' does not seems to be installed", &version, &tool)),
        },
        None => Err(anyhow!("You don't have a version configured for '{}' ({})", &exe_name, &tool)),
    }
}

fn configure_log(runtime: &RuntimeEnvironment) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str("debug")?
        .log_to_file(FileSpec::default().directory(&runtime.log_dir))
        .rotate(Criterion::Size(100_000), Naming::Numbers, Cleanup::KeepLogFiles(6))
        .append()
        .start()?)
}
