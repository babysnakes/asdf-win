use std::ffi::{OsStr, OsString};

use anyhow::{anyhow, Context, Result};
use log::debug;

use crate::{
    executable_context::ExecutableContext, plugins::plugin_manager::PluginManager, runtime::RuntimeEnvironment,
    shims::Shims, subcommand::exec, tool_versions::ToolVersions,
};

/// Representing a _shim_ command (name). Could be resolved (exists with it's
/// name) or unresolved (e.g. could be specified without extension).
pub enum Cmd<'a> {
    Resolved(&'a OsStr),
    UnResolved(&'a OsStr),
}

pub fn find_path_for_cmd(env: &RuntimeEnvironment, cmd: &str) -> Result<OsString> {
    let func = |ec: ExecutableContext| match ec.get_full_executable_path() {
        Some(path) => Ok(path.into_os_string()),
        None => Err(anyhow!(
            "{:?} does not exist in version '{}' of '{:?}'",
            &ec.cmd_name,
            &ec.version.to_string_lossy(),
            &ec.plugin.name
        )),
    };
    run_with_executable_context(env, &Cmd::UnResolved(OsStr::new(cmd)), None, func)
}

pub fn find_path_for_cmd_with_tool(env: &RuntimeEnvironment, cmd: &str, tool: &str) -> Result<OsString> {
    let func = |ec: ExecutableContext| match ec.get_full_executable_path() {
        Some(path) => Ok(path.into_os_string()),
        None => Err(anyhow!(
            "{:?} does not exist in version '{}' of '{:?}'",
            &ec.cmd_name,
            &ec.version.to_string_lossy(),
            &ec.plugin.name
        )),
    };
    let tool = OsStr::new(tool);

    run_with_executable_context(env, &Cmd::UnResolved(OsStr::new(cmd)), Some(tool), func)
}

pub fn execute_command<I, S>(env: &RuntimeEnvironment, cmd: &Cmd, args: I) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let func = |ec: ExecutableContext| {
        let command = ec.mk_command(args).ok_or_else(|| {
            anyhow!(
                "Command {:?} does not exist in {:?} version {}",
                ec.cmd_name,
                ec.plugin.name,
                ec.version.to_string_lossy()
            )
        })?;
        exec(command)
    };

    run_with_executable_context(env, cmd, None, func)
}

pub fn run_with_executable_context<F, T>(
    env: &RuntimeEnvironment,
    cmd: &Cmd,
    tool: Option<&OsStr>,
    func: F,
) -> Result<T>
where
    F: FnOnce(ExecutableContext) -> Result<T>,
{
    let pm = PluginManager::new(&env.plugins_dir);
    let shims = Shims::new(&env.shims_db, &env.installs_dir, &env.shims_dir, &env.shim_exe, &pm)?;
    let cmd_name = match *cmd {
        Cmd::Resolved(cmd) => OsString::from(cmd),
        Cmd::UnResolved(name) => {
            let context = format!("resolving command ({:?})", name);
            let resolved = shims
                .resolve_command(name)
                .context(context)?
                .ok_or_else(|| anyhow!("Could not find shim named: '{:?}'", name))?;
            debug!("Command '{:?}' resolved to: '{:?}'", name, resolved);
            resolved
        }
    };
    let tool = match tool {
        Some(tool) => tool.to_owned(),
        None => shims
            .find_tool(&cmd_name)?
            .ok_or_else(|| anyhow!("No tool configured for the command: {:?}", &cmd_name))?,
    };
    let plugin = pm.get_plugin(&tool).with_context(|| format!("Getting plugin for {:?}", tool))?;
    let tool_name = tool
        .to_str()
        .ok_or_else(|| anyhow!("Could not convert tool name {:?} to UTF-8. Are you using non UTF-8 charset?", &tool))?;
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, tool_name);
    let version = tvs.get_version()?.ok_or_else(|| anyhow!("No version configured for {:?}", &tool))?;
    let ec = ExecutableContext::new(&cmd_name, plugin, OsStr::new(&version), &env.installs_dir)
        .ok_or_else(|| anyhow!("Version '{version}' of '{:?}' is configured but not installed", tool))?;
    func(ec)
}
