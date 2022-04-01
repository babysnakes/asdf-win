use std::ffi::OsStr;

use anyhow::{anyhow, Context, Result};
use log::debug;

use crate::{
    executable_context::ExecutableContext, plugins::plugin_manager::PluginManager, runtime::RuntimeEnvironment,
    shims::Shims, subcommand::exec, tool_versions::ToolVersions,
};

/// Representing a _shim_ command (name). Could be resolved (exists with it's
/// name) or unresolved (e.g. could be specified without extension).
pub enum Cmd<'a> {
    Resolved(&'a str),
    UnResolved(&'a str),
}

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
    run_with_executable_context(env, &Cmd::UnResolved(cmd), None, func)
}

pub fn execute_command<I, S>(env: &RuntimeEnvironment, cmd: &Cmd, args: I) -> Result<i32>
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
    run_with_executable_context(env, cmd, None, func)
}

pub fn run_with_executable_context<F, T>(
    env: &RuntimeEnvironment,
    cmd: &Cmd,
    tool: Option<String>,
    func: F,
) -> Result<T>
where
    F: FnOnce(ExecutableContext) -> Result<T>,
{
    let pm = PluginManager::new(&env.plugins_dir);
    let shims = Shims::new(&env.shims_db, &env.installs_dir, &env.shims_dir, &env.shim_exe, &pm)?;
    let cmd_name = match cmd {
        Cmd::Resolved(cmd) => cmd.to_string(),
        Cmd::UnResolved(name) => {
            let context = format!("resolving command ({})", name);
            let resolved = shims
                .resolve_command(&name)
                .context(context)?
                .ok_or_else(|| anyhow!("Could not find shim named: '{name}'"))?;
            debug!("Command '{}' resolved to: '{}'", name, resolved);
            resolved
        }
    };
    let tool = match tool {
        Some(tool) => tool,
        None => shims
            .find_tool(&cmd_name)?
            .ok_or_else(|| anyhow!("No tool configured for the command: {}", &cmd_name))?,
    };
    let plugin = pm.get_plugin(&tool).with_context(|| format!("Getting plugin for {tool}"))?;
    let tvs = ToolVersions::new(&env.global_tool_versions_file, &env.current_dir, &tool);
    let version = tvs.get_version()?.ok_or_else(|| anyhow!("No version configured for {}", &tool))?;
    let ec = ExecutableContext::new(&cmd_name, plugin, &version, &env.installs_dir)
        .ok_or_else(|| anyhow!("Version '{version}' of '{tool}' is configured but not installed"))?;
    func(ec)
}
