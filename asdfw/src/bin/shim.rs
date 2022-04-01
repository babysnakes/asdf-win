use anyhow::{Context, Result};
use asdfw::common::{execute_command, Cmd};
use asdfw::runtime::RuntimeEnvironment;
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
    // Todo: Convert to OsString?
    let exe_name = me.file_name().unwrap().to_str().unwrap();
    let args = env::args().skip(1);
    let runtime = RuntimeEnvironment::new()?;
    if env::var(DEBUG_VARIABLE).is_ok() {
        configure_log(&runtime)?;
    };
    execute_command(&runtime, &Cmd::UnResolved(exe_name), args)
}

fn configure_log(runtime: &RuntimeEnvironment) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str("debug")?
        .log_to_file(FileSpec::default().directory(&runtime.log_dir))
        .rotate(Criterion::Size(100_000), Naming::Numbers, Cleanup::KeepLogFiles(6))
        .append()
        .start()?)
}
