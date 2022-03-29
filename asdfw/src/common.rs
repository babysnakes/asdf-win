use std::ffi::OsStr;

use anyhow::{anyhow, Context, Result};
use log::debug;

use crate::{
    executable_context::ExecutableContext, plugins::plugin_manager::PluginManager, runtime::RuntimeEnvironment,
    shims::Shims, subcommand::exec, tool_versions::ToolVersions,
};

// Todo: return someting else,not string.
pub fn find_path_for_cmd(env: &RuntimeEnvironment, cmd: &str) -> Result<String> {
    let func = |ec: ExecutableContext| match ec.get_full_executable_path() {
        Some(path) => {
            let path_str = path.to_str().ok_or_else(|| {
                anyhow!("Couldn't convert '{:?}' to UTF8 string. Are you using non UTF8 file system?", path)
            })?;
            Ok(path_str.to_owned())
        }
        None => Err(anyhow!(
            "{} does not exist in version '{}' of '{}'",
            &ec.cmd_name,
            &ec.version,
            &ec.plugin.name
        )),
    };
    run_with_executable_context(env, cmd, func)
}

pub fn execute_command<I, S>(env: &RuntimeEnvironment, cmd: &str, args: I) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let func = |ec: ExecutableContext| {
        let command = ec.mk_command(args).ok_or_else(|| {
            anyhow!("Command {} does not exist in {} version {}", ec.cmd_name, ec.plugin.name, ec.version)
        })?;
        exec(command)
    };
    run_with_executable_context(env, cmd, func)
}

pub fn run_with_executable_context<F, T>(env: &RuntimeEnvironment, cmd: &str, func: F) -> Result<T>
where
    F: FnOnce(ExecutableContext) -> Result<T>,
{
    let pm = PluginManager::new(&env.plugins_dir);
    let shims = Shims::new(&env.shims_db, &env.installs_dir, &env.shims_dir, &env.shim_exe, &pm)?;
    let context = format!("resolving command ({})", &cmd);
    let cmd_name = shims
        .resolve_command(cmd)
        .context(context)?
        .ok_or_else(|| anyhow!("Could not find shim named: '{cmd}'"))?;
    debug!("Command '{}' resolved to: '{}'", &cmd, &cmd_name);
    let tool = shims
        .find_tool(&cmd_name)?
        .ok_or_else(|| anyhow!("No tool configured for the command: {}", &cmd_name))?;
    let plugin = pm.get_plugin(&tool).with_context(|| format!("Getting plugin for {tool}"))?;
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, &tool);
    let version = tvs.get_version()?.ok_or_else(|| anyhow!("No version configured for {}", &tool))?;
    let ec = ExecutableContext::new(&cmd_name, plugin, &version, &env.installs_dir)
        .ok_or_else(|| anyhow!("Version '{version}' of '{tool}' is configured but not installed"))?;
    func(ec)
}
