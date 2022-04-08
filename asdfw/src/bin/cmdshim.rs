use anyhow::{anyhow, Context, Result};
use asdfw::common::{execute_cmd_script, Cmd};
use asdfw::runtime::RuntimeEnvironment;
use flexi_logger::*;
use std::ffi::OsStr;
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
    let mut args = env::args().skip(1);
    let exe_name = args.next().ok_or_else(|| anyhow!("Command to run not supplied!"))?;
    let tool = args.next().ok_or_else(|| anyhow!("Tool not supplied!"))?;
    let runtime = RuntimeEnvironment::new()?;
    if env::var(DEBUG_VARIABLE).is_ok() {
        configure_log(&runtime, &exe_name)?;
    };

    execute_cmd_script(&runtime, &Cmd::Resolved(OsStr::new(&exe_name)), args, &tool)
}

fn configure_log(runtime: &RuntimeEnvironment, basename: &str) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str("debug")?
        .log_to_file(FileSpec::default().basename(basename).directory(&runtime.log_dir))
        .rotate(Criterion::Size(100_000), Naming::Numbers, Cleanup::KeepLogFiles(6))
        .append()
        .start()?)
}
