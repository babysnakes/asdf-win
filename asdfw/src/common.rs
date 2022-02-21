use anyhow::{anyhow, Context, Result};
use log::debug;

use crate::{runtime::RuntimeEnvironment, shims::Shims, tool_versions::ToolVersions};

pub fn find_path_for_cmd(env: &RuntimeEnvironment, cmd: &str) -> Result<String> {
    let shims = Shims::new(&env.shims_db, &env.installs_dir, &env.shims_dir, &env.shim_exe)?;
    let context = format!("resolving command ({})", &cmd);
    let cmd_name = shims.resolve_command(cmd).context(context)?.unwrap_or_else(|| cmd.to_string());
    debug!("Command '{}' resolved to: '{}'", &cmd, &cmd_name);
    let tool = shims
        .find_plugin(&cmd_name)?
        .ok_or_else(|| anyhow!("No tool configured for the command: {}", &cmd_name))?;
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, &tool);
    let version = tvs.get_version()?.ok_or_else(|| anyhow!("No version configured for {}", &tool))?;
    match shims.get_full_executable_path(&cmd_name, &tool, &version)? {
        Some(path) => {
            let path_str = path.to_str().ok_or_else(|| {
                anyhow!("Couldn't convert '{:?}' to UTF8 string. Are you using non UTF8 file system?", path)
            })?;
            Ok(path_str.to_owned())
        }
        None => Err(anyhow!(
            "Version '{}' of '{}' configured but not installed (tool: {})",
            &version,
            &cmd_name,
            &tool
        )),
    }
}
