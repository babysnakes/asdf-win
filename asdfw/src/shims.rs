use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use stripmargin::StripMargin;

use crate::plugins::plugin_manager::PluginManager;

pub type ShimsDB = HashMap<OsString, ShimData>;

/// The Shims struct contains data required for handling shims.
pub struct Shims<'a> {
    path: &'a Path,
    tools_install_dir: &'a Path,
    shims_dir: &'a Path,
    shim_exe: &'a Path,
    plugin_manager: &'a PluginManager<'a>,
    extensions: HashMap<&'static str, ShimType>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub enum ShimType {
    ExeShim,
    CmdShim,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ShimData {
    pub tool: OsString,
    pub tipe: ShimType,
}

impl<'a> Shims<'a> {
    /// Create a new Shims struct from the provided db path and installations
    /// directory.
    pub fn new(
        db_path: &'a Path,
        tools_install_dir: &'a Path,
        shims_dir: &'a Path,
        shim_exe: &'a Path,
        plugin_manager: &'a PluginManager<'a>,
    ) -> Result<Self> {
        if !tools_install_dir.is_dir() {
            return Err(anyhow!(
                "Supplied tools install dir ({:?}) is not an existing directory",
                tools_install_dir
            ));
        };
        Ok(Shims {
            path: db_path,
            tools_install_dir,
            shims_dir,
            shim_exe,
            plugin_manager,
            extensions: HashMap::from([("exe", ShimType::ExeShim), ("cmd", ShimType::CmdShim)]),
        })
    }

    fn load_db(&self) -> Result<ShimsDB> {
        let contents = fs::read(self.path)?;
        bincode::deserialize(&contents).map_err(|err| anyhow!("Error deserializing ShimsDB: {}", err))
    }

    /// Save the provided shims db to a file.
    pub fn save_db(&self, db: &ShimsDB) -> Result<()> {
        let serialized = bincode::serialize(db)?;
        fs::write(self.path, &serialized)?;
        info!("Successfully saved db");
        Ok(())
    }

    /// Resolve executable name as shim even if entered without extension.
    pub fn resolve_command(&self, exe: &OsStr) -> Result<Option<OsString>> {
        for entry in fs::read_dir(&self.shims_dir)? {
            let name = entry?.file_name();
            if exe == name {
                return Ok(Some(name));
            }
            let path = Path::new(exe);
            for (ext, _) in self.extensions.iter() {
                let with_ext = path.with_extension(ext);
                if with_ext.as_os_str() == name {
                    return Ok(Some(name));
                }
            }
        }
        Ok(None)
    }

    /// Find a plugin which owns this exe
    pub fn find_tool(&self, exe: &OsStr) -> Result<Option<OsString>> {
        let shims = self.load_db()?;
        Ok(shims.get(exe).map(|s| s.tool.to_owned()))
    }

    /// Generates all required shims. Cleans up the shims directory before if desired.
    pub fn create_shims(&self, cleanup: bool) -> Result<()> {
        // Fix: add support for CMD shims
        if cleanup {
            debug!("resetting shims directory");
            fs::remove_dir_all(self.shims_dir).context("cleaning up shims directory")?;
            fs::create_dir(self.shims_dir).context("recreating shims directory after cleanup")?;
        }
        let db = self.load_db()?;
        for (exe, data) in db {
            let target = self.shims_dir.join(&exe);
            debug!("Creating shim for {:?}", &exe);
            match data.tipe {
                ShimType::ExeShim => {
                    fs::copy(&self.shim_exe, target).context(format!("creating shim for {:?}", &exe))?;
                }
                ShimType::CmdShim => {
                    let content = format!(
                        r#"@ECHO OFF
                          |                          
                          |SETLOCAL
                          |
                          |FOR /F "delims=" %%F IN ('CALL cmdshim.exe {:?} {:?}') DO (
                          |    SET commandToRun=%%F
                          |)
                          |
                          |if "%commandToRun%" == "" (
                          |    exit 1
                          |)
                          |
                          |"%commandToRun%" %*
                          |"#,
                        &exe, &data.tool,
                    )
                    .strip_margin();
                    fs::write(&target, &content)?;
                }
            }
        }
        Ok(())
    }

    pub fn generate_db_from_installed_tools(&self) -> Result<ShimsDB> {
        let mut db: ShimsDB = HashMap::new();

        for entry in fs::read_dir(self.tools_install_dir)? {
            let entry = entry?;
            let tool = entry.file_name();
            for version in fs::read_dir(entry.path())? {
                let version = version?;
                if version.path().is_dir() {
                    let plugin = &self
                        .plugin_manager
                        .get_plugin(&tool)
                        .with_context(|| format!("loading plugin for {:?}", &tool))?;
                    for bin_dir in &plugin.config.bin_dirs {
                        let mut path = version.path();
                        path.push(bin_dir);
                        for exe in fs::read_dir(path)? {
                            let exe = exe?;
                            if let Some(tipe) = self.requires_shim(exe.path().extension()) {
                                let exe_name = exe.file_name();
                                let shim_data = ShimData {
                                    tool: tool.clone(),
                                    tipe,
                                };
                                let old_value = db.insert(exe_name.clone(), shim_data);
                                if let Some(value) = old_value {
                                    if value.tool != tool {
                                        return Err(anyhow!(
                                            "{:?} appears in two tools: {:?} and {:?}",
                                            &exe_name,
                                            &tool,
                                            &value.tool
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(db)
    }

    fn requires_shim(&self, extension: Option<&OsStr>) -> Option<ShimType> {
        for (item, tipe) in &self.extensions {
            if Some(OsStr::new(item)) == extension {
                return Some(*tipe);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::plugins::plugin::PLUGIN_FILENAME;

    use super::*;
    use assert_fs::{fixture::ChildPath, prelude::*, TempDir};
    use rstest::rstest;
    use std::fs::OpenOptions;

    struct TestPaths {
        tools_install_dir: ChildPath,
        db_path: ChildPath,
        shims_dir: ChildPath,
        shim_exe: ChildPath,
        plugins_dir: ChildPath,
    }

    fn test_paths(root: &TempDir) -> TestPaths {
        let tools_install_dir = root.child("installs");
        tools_install_dir.create_dir_all().unwrap();
        let shims_dir = root.child("shims");
        shims_dir.create_dir_all().unwrap();
        let db_path = root.child("shims.db");
        let shim_exe = root.child("shim.exe");
        shim_exe.touch().unwrap();
        let plugins_dir = root.child("plugins");
        TestPaths {
            tools_install_dir,
            db_path,
            shims_dir,
            shim_exe,
            plugins_dir,
        }
    }

    fn mk_exe_shim_data(tool: &str) -> ShimData {
        ShimData {
            tool: OsString::from(tool),
            tipe: ShimType::ExeShim,
        }
    }

    fn test_data() -> ShimsDB {
        HashMap::from([
            (OsString::from("kubectl.exe"), mk_exe_shim_data("kubectl")),
            (OsString::from("docker.exe"), mk_exe_shim_data("docker")),
            (OsString::from("minikube.exe"), mk_exe_shim_data("minikube")),
            (OsString::from("kubectx.exe"), mk_exe_shim_data("kubectx")),
            (OsString::from("kubens.exe"), mk_exe_shim_data("kubectx")),
        ])
    }

    /// Generates a `mytool` tool plugin (version 1.0) and installation with two
    /// binaries (and custom bin dirs):
    /// - mytool\1.0\bin1\tool-bin1.exe
    /// - mytool\1.0\some\bin2\tool-bin2.exe
    /// -
    fn custom_bin_dirs_fixture(tmpdir: &TempDir) -> TestPaths {
        let tool = "mytool";
        let paths = test_paths(tmpdir);
        let my_plugin_dir = paths.plugins_dir.child(tool);
        my_plugin_dir.create_dir_all().unwrap();
        let my_plugin_config = my_plugin_dir.child(PLUGIN_FILENAME);
        my_plugin_config.write_str("---\nbin_dirs:\n  - bin1\n  - some\\bin2").unwrap();
        let tooldir = paths.tools_install_dir.child(tool).child("1.0");
        let bin1_dir = tooldir.child("bin1");
        let bin2_dir = tooldir.child("some").child("bin2");
        bin1_dir.create_dir_all().unwrap();
        bin2_dir.create_dir_all().unwrap();
        let bin1 = bin1_dir.child("tool-bin1.exe");
        let bin2 = bin2_dir.child("tool-bin2.exe");
        bin1.touch().unwrap();
        bin2.touch().unwrap();
        paths
    }

    #[test]
    fn save_and_load_shims_db() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        shims.save_db(&db).unwrap();
        let loaded = shims.load_db().unwrap();
        assert_eq!(db, loaded);
    }

    #[rustfmt::skip]
    #[rstest]
    #[case(vec!["hello.exe", "world.exe"], OsStr::new("hello.exe"), Some(OsString::from("hello.exe")), "exact match")]
    #[case(vec!["hello.exe", "world.exe"], OsStr::new("hello"), Some(OsString::from("hello.exe")), "missing extension")]
    #[case(vec!["hello.exe", "world.exe"], OsStr::new("what.exe"), None, "invalid command")]
    fn resolve_command_tests(
        #[case] existing_shims: Vec<&str>,
        #[case] exe: &OsStr,
        #[case] expected: Option<OsString>,
        #[case] msg: &str,
    ) {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        for n in existing_shims {
            paths.shims_dir.child(&n).touch().unwrap();
        }
        let result = shims.resolve_command(exe).unwrap();
        assert_eq!(result, expected, "test case: {}", &msg);
    }

    #[test]
    fn find_tool_existing_tool_returns_valid_tool() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        shims.save_db(&db).unwrap();
        let result = shims.find_tool(OsStr::new("kubens.exe")).unwrap();
        assert_eq!(result, Some(OsString::from("kubectx")));
    }

    #[test]
    fn find_tool_with_invalid_tool_returns_none() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        shims.save_db(&db).unwrap();
        let result = shims.find_tool(OsStr::new("mycmd")).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_should_succeed() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("docker").child("v1.17").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("docker").child("v1.17").child("bin").child("docker.exe").touch().unwrap();
        paths.tools_install_dir.child("docker").child("v1.19").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("docker").child("v1.19").child("bin").child("docker.exe").touch().unwrap();
        paths.tools_install_dir.child("minikube").child("2.5").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("minikube").child("2.5").child("bin").child("minikube.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectx").child("0.12").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectx").child("0.12").child("bin").child("kubectx.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectx").child("0.12").child("bin").child("kubens.exe").touch().unwrap();
        paths.tools_install_dir.child("nodejs").child("1.12").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("nodejs").child("1.12").child("bin").child("npm.cmd").touch().unwrap();
        let mut db = test_data();
        db.insert(OsString::from("npm.cmd"), ShimData { tool: OsString::from("nodejs"), tipe: ShimType::CmdShim });

        let generated = shims.generate_db_from_installed_tools().unwrap();
        assert_eq!(db, generated);
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_should_not_include_files_without_valid_extension() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").child("kubectl.txt").touch().unwrap();
        let generated = shims.generate_db_from_installed_tools().unwrap();
        assert!(!generated.contains_key(OsStr::new("kubectl.txt")), "should not contain files with wrong extension");
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_with_same_executable_name_in_two_tools_should_fail() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").create_dir_all().unwrap();
        // !! The executable created below should trigger an error:
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").child("kubens.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectx").child("0.12").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectx").child("0.12").child("bin").child("kubectx.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectx").child("0.12").child("bin").child("kubens.exe").touch().unwrap();

        let res = shims.generate_db_from_installed_tools().unwrap_err();
        let err = format!("{:?}", res);
        assert!(err.contains("kubens.exe"), "Wrong error was triggered ({:?}), should contain 'kubens.exe'", err);
    }

    #[test]
    #[rustfmt::skip]
    fn create_shims_without_cleanup_should_create_shims_that_exists_in_the_db() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        shims.save_db(&db).unwrap();
        shims.create_shims(false).unwrap();
        db.keys().for_each(|k| {
            assert!(shims.shims_dir.join(k).exists(), "shim '{:?}' does not exist", &k);
        });
        assert_eq!(shims.shims_dir.read_dir().unwrap().count(), 5);
    }

    #[test]
    #[rustfmt::skip]
    fn create_shims_should_produce_correct_cmd_script_shims() {
        let mut db = test_data();
        db.insert(OsString::from("npm.cmd"), ShimData { tool: OsString::from("nodejs"), tipe: ShimType::CmdShim });
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        shims.save_db(&db).unwrap();
        shims.create_shims(false).unwrap();
        assert_eq!(shims.shims_dir.read_dir().unwrap().count(), 6);
        let cmdshim = shims.shims_dir.join("npm.cmd");
        let content = fs::read_to_string(&cmdshim).unwrap();
        dbg!(&content);
        assert!(content.contains("cmdshim.exe"), "cmd shim should contain call to cmdshim.exe");
        assert!(content.contains("npm.cmd"), "cmd shim should contain running script");
        assert!(content.contains("nodejs"), "cmd shim should contain tool");
    }

    #[test]
    #[rustfmt::skip]
    fn create_shims_without_cleanup_leaves_dangling_shims_in_place() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        let dangling = shims.shims_dir.join("invalid.exe");
        OpenOptions::new().create(true).write(true).open(&dangling).unwrap();
        shims.save_db(&db).unwrap();
        shims.create_shims(false).unwrap();
        assert_eq!(shims.shims_dir.read_dir().unwrap().count(), 6);
        assert!(dangling.exists(), "dangling file was deleted in create shims without cleanup");
    }

    #[test]
    #[rustfmt::skip]
    fn create_shims_with_cleanup_removes_dangling_shims() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        let dangling = shims.shims_dir.join("invalid.exe");
        OpenOptions::new().create(true).write(true).open(&dangling).unwrap();
        shims.save_db(&db).unwrap();
        shims.create_shims(true).unwrap();
        assert!(!dangling.exists(), "dangling file was not deleted in create shims with cleanup");
        assert_eq!(shims.shims_dir.read_dir().unwrap().count(), 5);
    }

    #[test]
    fn generate_shims_db_with_plugin_that_contains_multiple_bin_dirs_should_include_bins_from_all_dirs() {
        let tmpdir = TempDir::new().unwrap();
        let paths = custom_bin_dirs_fixture(&tmpdir);
        let pm = PluginManager::new(&paths.plugins_dir);
        let shims =
            Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe, &pm).unwrap();
        let db = shims.generate_db_from_installed_tools().unwrap();
        assert!(
            db.contains_key(OsStr::new("tool-bin1.exe")),
            "should contain executables from bin1 dir"
        );
        assert!(
            db.contains_key(OsStr::new("tool-bin2.exe")),
            "should contain executables from bin2 dir"
        );
    }
}
