use std::{
    env::var_os,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

use crate::plugins::plugin::{EnvVar, EnvVarValue, Plugin};
use log::debug;

#[derive(Debug)]
pub struct ExecutableContext<'a> {
    pub cmd_name: &'a OsStr,
    pub plugin: Plugin<'a>,
    pub version: &'a OsStr,
    tool_install_root: PathBuf,
}

impl<'a> ExecutableContext<'a> {
    pub fn new(cmd_name: &'a OsStr, plugin: Plugin<'a>, version: &'a OsStr, tools_install_dir: &Path) -> Option<Self> {
        let tool_install_root: PathBuf =
            [tools_install_dir, Path::new(plugin.name), Path::new(version)].iter().collect();
        if tool_install_root.exists() {
            Some(ExecutableContext {
                cmd_name,
                plugin,
                version,
                tool_install_root,
            })
        } else {
            None
        }
    }

    /// Extracts the full executable path from `self.cmd_name`. Return `None`
    /// if a command by that name is not found in the current version.
    pub fn get_full_executable_path(&self) -> Option<PathBuf> {
        debug!("Searching for full executable path for shim: {:?}", self.cmd_name);
        for dir in &self.plugin.config.bin_dirs {
            let assumed: PathBuf =
                [&self.tool_install_root, Path::new(&dir), Path::new(self.cmd_name)].iter().collect();
            debug!("Checking existance of assumed shim target: {:?}", &assumed);
            if assumed.exists() {
                return Some(assumed);
            }
        }
        None
    }

    /// Creates a `process::Command` from self (plus the provided args). Returns
    /// `None` if `self.cmd_name` does not point to any existing command.
    pub fn mk_command<I, S>(&self, args: I) -> Option<Command>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let envs = self.env_vars_to_envs(&self.plugin.config.env_vars);
        let cmd = self.get_full_executable_path()?;
        let mut command = Command::new(&cmd);
        command.args(args).envs(envs);
        Some(command)
    }

    fn env_vars_to_envs(&self, env_vars: &[EnvVar]) -> Vec<(OsString, OsString)> {
        env_vars.iter().map(|ev| self.parse_env_var(ev)).collect()
    }

    fn parse_env_var(&self, envvar: &EnvVar) -> (OsString, OsString) {
        let parse_value = |value: &EnvVarValue| match value {
            EnvVarValue::Value { value } => OsString::from(value),
            EnvVarValue::RelativeInstallPath { relative_inst_path } => {
                let path = self.tool_install_root.join(relative_inst_path);
                path.as_os_str().to_owned()
            }
        };

        let value = match &envvar.overriding_name {
            None => parse_value(&envvar.value),
            Some(var_name) => match var_os(var_name) {
                Some(value) => value,
                None => parse_value(&envvar.value),
            },
        };

        (OsString::from(&envvar.name), value)
    }
}

#[cfg(test)]
mod tests {
    use crate::plugins::plugin::PLUGIN_FILENAME;

    use super::*;
    use assert_fs::{fixture::ChildPath, prelude::*, TempDir};
    use stripmargin::StripMargin;

    struct Paths {
        plugin_dir: ChildPath,
        installs_dir: ChildPath,
    }

    struct ExecutableContextFuxture<'a> {
        ec: ExecutableContext<'a>,
        tool_dir: ChildPath,
    }

    fn required_dirs(root: &TempDir, tool: &OsStr) -> Paths {
        let plugin_dir = root.child(&tool);
        let installs_dir = root.child("installs");
        plugin_dir.create_dir_all().unwrap();
        installs_dir.create_dir_all().unwrap();
        Paths {
            plugin_dir,
            installs_dir,
        }
    }

    fn fixture_executable_context<'a>(
        root: &TempDir,
        cmd: &'a str,
        tool: &'a str,
        version: &'a str,
        plugin_yml: Option<&str>,
    ) -> ExecutableContextFuxture<'a> {
        let cmd = OsStr::new(cmd);
        let version = OsStr::new(version);
        let tool = OsStr::new(tool);
        let paths = required_dirs(root, tool);
        let tool_dir = paths.installs_dir.child(&tool).child(&version);
        if let Some(txt) = plugin_yml {
            let conf = paths.plugin_dir.child(PLUGIN_FILENAME);
            conf.write_str(txt).unwrap();
        }
        tool_dir.create_dir_all().unwrap();
        let plugin = Plugin::new(tool, paths.plugin_dir.to_path_buf()).unwrap();
        let ec = ExecutableContext::new(cmd, plugin, version, &paths.installs_dir).unwrap();
        ExecutableContextFuxture { ec, tool_dir }
    }

    #[test]
    fn new_computes_tool_install_root_correctly() {
        let tool = OsStr::new("mytool");
        let version = OsStr::new("0.1");
        let tmpdir = TempDir::new().unwrap();
        let paths = required_dirs(&tmpdir, tool);
        let tool_dir = paths.installs_dir.child(&tool).child(&version);
        tool_dir.create_dir_all().unwrap();
        let plugin = Plugin::new(tool, paths.plugin_dir.to_path_buf()).unwrap();
        let cmd_name = OsStr::new("cmd.exe");
        let result = ExecutableContext::new(cmd_name, plugin, version, &paths.installs_dir).unwrap();
        assert_eq!(result.tool_install_root, tool_dir.to_path_buf());
    }

    #[test]
    fn new_returns_error_if_tool_directory_does_not_exist() {
        let tool = OsStr::new("mytool");
        let version = OsStr::new("0.1");
        let tmpdir = TempDir::new().unwrap();
        let paths = required_dirs(&tmpdir, tool);
        let plugin = Plugin::new(tool, paths.plugin_dir.to_path_buf()).unwrap();
        let cmd_name = OsStr::new("cmd.exe");
        let result = ExecutableContext::new(cmd_name, plugin, version, &paths.installs_dir);
        assert!(result.is_none(), "should return none if install root does not exist");
    }

    #[test]
    fn get_full_executable_path_when_version_exists_returns_path() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, None);
        let bindir = fixture.tool_dir.child("bin");
        bindir.create_dir_all().unwrap();
        let exe = bindir.child(&cmd);
        exe.touch().unwrap();
        let result = fixture.ec.get_full_executable_path().unwrap();
        let expected = exe.to_path_buf();
        assert_eq!(result, expected);
    }

    #[test]
    fn get_full_executable_path_when_not_exists_returns_none() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, None);
        // do not create binary dir and file
        let result = fixture.ec.get_full_executable_path();
        assert!(result.is_none(), "if executable not found should return none");
    }

    #[test]
    fn get_full_executable_path_with_non_default_path_should_work() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let yml = r#"---
            |bin_dirs:
            |  - some\dir
            |"#
        .strip_margin();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, Some(&yml));
        let bindir = fixture.tool_dir.child("some").child("dir");
        bindir.create_dir_all().unwrap();
        let exe = bindir.child(&cmd);
        exe.touch().unwrap();
        let result = fixture.ec.get_full_executable_path().unwrap();
        let expected = exe.to_path_buf();
        assert_eq!(result, expected);
    }

    #[test]
    fn get_full_executable_path_with_nultiple_paths_should_loop_through_path() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let yml = r#"---
            |bin_dirs:
            |  - some\dir
            |  - other\bin
            |"#
        .strip_margin();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, Some(&yml));
        let bindir1 = fixture.tool_dir.child("some").child("dir");
        bindir1.create_dir_all().unwrap();
        let bindir2 = fixture.tool_dir.child("other").child("bin");
        bindir2.create_dir_all().unwrap();
        let exe = bindir2.child(&cmd);
        exe.touch().unwrap();
        let result = fixture.ec.get_full_executable_path().unwrap();
        let expected = exe.to_path_buf();
        assert_eq!(result, expected);
    }

    #[test]
    fn mk_command_sets_the_program_correctly() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, None);
        let bindir = fixture.tool_dir.child("bin");
        bindir.create_dir_all().unwrap();
        let exe = bindir.child(&cmd);
        exe.touch().unwrap();
        let args: Vec<&str> = vec![];
        let command = fixture.ec.mk_command(args).unwrap();
        assert_eq!(command.get_program(), exe.as_os_str());
    }

    #[test]
    fn mk_command_returns_none_if_can_not_find_fll_executable_path() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, None);
        let args: Vec<&str> = vec![];
        assert!(fixture.ec.mk_command(args).is_none(), "should return none on missing executable");
    }

    #[test]
    fn mk_command_sets_the_args_correctly() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, None);
        let bindir = fixture.tool_dir.child("bin");
        bindir.create_dir_all().unwrap();
        let exe = bindir.child(&cmd);
        exe.touch().unwrap();
        let args: Vec<&str> = vec!["-f", "some"];
        let command = fixture.ec.mk_command(&args).unwrap();
        let mut result = command.get_args();
        assert_eq!(result.next().unwrap(), args[0]);
        assert_eq!(result.next().unwrap(), args[1]);
    }

    #[test]
    fn mk_command_without_args_behaves_correctly() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, None);
        let bindir = fixture.tool_dir.child("bin");
        bindir.create_dir_all().unwrap();
        let exe = bindir.child(&cmd);
        exe.touch().unwrap();
        let args: Vec<&str> = vec![];
        let command = fixture.ec.mk_command(&args).unwrap();
        let mut result = command.get_args();
        assert!(result.next().is_none(), "expected empty args, got {:?}", result);
    }

    #[test]
    fn mk_command_sets_environment_variables_if_requested() {
        let tool = "mytool";
        let version = "0.1";
        let cmd = "cmd.exe";
        let yml = r#"---
            |env_vars:
            |  - name: MYENV
            |    value:
            |      value: "A Value"
            |"#
        .strip_margin();
        let tmpdir = TempDir::new().unwrap();
        let fixture = fixture_executable_context(&tmpdir, cmd, tool, version, Some(&yml));
        let bindir = fixture.tool_dir.child("bin");
        bindir.create_dir_all().unwrap();
        let exe = bindir.child(&cmd);
        exe.touch().unwrap();
        let args: Vec<&str> = vec![];
        let command = fixture.ec.mk_command(&args).unwrap();
        let mut envs = command.get_envs();
        let expected = (OsStr::new("MYENV"), Some(OsStr::new("A Value")));
        let first_env = envs.next().unwrap();
        assert_eq!(first_env, expected);
    }

    #[test]
    fn parse_env_var_returns_provided_simple_values() {
        let tmpdir = TempDir::new().unwrap();
        let ecf = fixture_executable_context(&tmpdir, "cmd.exe", "tool", "0.1", None);
        let ev = EnvVar {
            name: "MY_VAR".to_string(),
            overriding_name: None,
            value: EnvVarValue::Value {
                value: "A Value".to_string(),
            },
        };
        let env = ecf.ec.parse_env_var(&ev);
        assert_eq!(env.0, OsString::from("MY_VAR"));
        assert_eq!(env.1, OsString::from("A Value"));
    }

    #[test]
    fn parse_env_var_parses_relative_paths_correctly() {
        let relative_path = r"some\path";
        let tmpdir = TempDir::new().unwrap();
        let ecf = fixture_executable_context(&tmpdir, "cmd.exe", "tool", "0.1", None);
        let ev = EnvVar {
            name: "MY_VAR".to_string(),
            overriding_name: None,
            value: EnvVarValue::RelativeInstallPath {
                relative_inst_path: relative_path.to_string(),
            },
        };
        let env = ecf.ec.parse_env_var(&ev);
        assert_eq!(env.0, OsString::from("MY_VAR"));
        let path = env.1.to_str().unwrap();
        assert!(
            path.ends_with(&relative_path),
            "Expected relative path to end with provided value, got: '{path}'"
        );
        let prefix = ecf.tool_dir.to_str().unwrap();
        assert!(
            path.starts_with(prefix),
            "Expected relative path to stat with install dir, got: '{path}'"
        );
    }

    #[test]
    fn parse_env_var_parses_overrides_correctly() {
        let tmpdir = TempDir::new().unwrap();
        let ecf = fixture_executable_context(&tmpdir, "cmd.exe", "tool", "0.1", None);
        let overriding_var_name = "OVERRIDING_MY_VAR";
        let overriding_value = "Overriding Value";
        let ev = EnvVar {
            name: "MY_VAR".to_string(),
            overriding_name: Some(overriding_var_name.to_string()),
            value: EnvVarValue::Value {
                value: "A Value".to_string(),
            },
        };
        temp_env::with_var(overriding_var_name, Some(overriding_value), || {
            let env = ecf.ec.parse_env_var(&ev);
            assert_eq!(env.0, OsString::from("MY_VAR"));
            assert_eq!(env.1, OsString::from(overriding_value));
        });
    }

    #[test]
    fn env_vars_to_envs_works_correctly() {
        let tmpdir = TempDir::new().unwrap();
        let ecf = fixture_executable_context(&tmpdir, "cmd.exe", "tool", "0.1", None);
        let evs = vec![
            EnvVar {
                name: "MY_VAR".to_string(),
                overriding_name: None,
                value: EnvVarValue::Value {
                    value: "A Value".to_string(),
                },
            },
            EnvVar {
                name: "MY_VAR2".to_string(),
                overriding_name: Some("NOSUCHVAR".to_string()),
                value: EnvVarValue::Value {
                    value: "A Value2".to_string(),
                },
            },
        ];
        let result = ecf.ec.env_vars_to_envs(&evs);
        let expected = vec![
            (OsString::from("MY_VAR"), OsString::from("A Value")),
            (OsString::from("MY_VAR2"), OsString::from("A Value2")),
        ];
        assert_eq!(result, expected);
    }
}
