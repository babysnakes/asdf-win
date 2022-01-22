use anyhow::{anyhow, Context, Result};
use asdfw::runtime::RuntimeEnvironment;
use asdfw::shims::Shims;
use asdfw::subcommand::exec;
use asdfw::tool_versions::ToolVersions;
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
    if let Ok(_) = env::var(DEBUG_VARIABLE) {
        configure_log(&runtime)?;
    };
    let shims = Shims::new(&runtime.shims_db, &runtime.installs_dir)?;
    let tool = shims
        .find_plugin(&exe_name)?
        .ok_or(anyhow!("No plugin by the name: {}", &exe_name))?;
    let tool_versions = ToolVersions::new(
        &runtime.global_tool_versions_file,
        &runtime.current_dir,
        &tool,
    );
    match tool_versions.get_version()? {
        Some(version) => {
            let cmd = shims.get_full_executable_path(&exe_name, &tool, &version)?;
            exec(&cmd, args)
        }
        None => Err(anyhow!(
            "You don't have a version configured for '{}' ({})",
            &exe_name,
            &tool
        )),
    }
}

fn configure_log(runtime: &RuntimeEnvironment) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str("debug")?
        .log_to_file(FileSpec::default().directory(&runtime.log_dir))
        .rotate(
            Criterion::Size(100_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(6),
        )
        .append()
        .start()?)
}
