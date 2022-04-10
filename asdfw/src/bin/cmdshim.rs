use anyhow::{anyhow, Context, Result};
use asdfw::common::find_path_for_cmd_with_tool;
use asdfw::runtime::RuntimeEnvironment;
use flexi_logger::*;
use log::debug;
use std::env;

const ERROR_PREFIX: &str = "ASDFW ERROR";
const DEBUG_VARIABLE: &str = "ASDFW_DEBUG_SHIM";

fn main() -> Result<()> {
    run().context(ERROR_PREFIX)
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let exe_name = args.next().ok_or_else(|| anyhow!("Command to run not supplied!"))?;
    let tool = args.next().ok_or_else(|| anyhow!("Tool not supplied!"))?;
    let runtime = RuntimeEnvironment::new()?;
    if env::var(DEBUG_VARIABLE).is_ok() {
        configure_log(&runtime, &exe_name)?;
    };
    let full_path = find_path_for_cmd_with_tool(&runtime, &exe_name, &tool)?;
    debug!("full path is: {:?}", full_path);
    let printable_path = full_path.to_str().ok_or_else(|| {
        anyhow!(
            "Could not convert tool path {:?} to UTF-8. Are you using non UTF-8 charset?",
            &full_path
        )
    })?;

    print!("{}", printable_path);
    Ok(())
}

fn configure_log(runtime: &RuntimeEnvironment, basename: &str) -> Result<LoggerHandle> {
    Ok(Logger::try_with_str("debug")?
        .log_to_file(FileSpec::default().basename(basename).directory(&runtime.log_dir))
        .rotate(Criterion::Size(100_000), Naming::Numbers, Cleanup::KeepLogFiles(6))
        .append()
        .start()?)
}
