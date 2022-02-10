use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const EXTENSIONS: &'static [&str] = &["exe"];

pub type ShimsDB = HashMap<String, String>;

/// The Shims struct contains data required for handling shims.
pub struct Shims<'a> {
    path: &'a Path,
    tools_install_dir: &'a Path,
    shims_dir: &'a Path,
    shim_exe: &'a Path,
}

impl<'a> Shims<'a> {
    /// Create a new Shims struct from the provided db path and installations
    /// directory.
    pub fn new(
        db_path: &'a Path,
        tools_install_dir: &'a Path,
        shims_dir: &'a Path,
        shim_exe: &'a Path,
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
        })
    }

    fn load_db(&self) -> Result<ShimsDB> {
        let contents = fs::read(self.path)?;
        bincode::deserialize(&contents)
            .map_err(|err| anyhow!("Error deserializing ShimsDB: {}", err))
    }

    /// Save the provided shims db to a file.
    pub fn save_db(&self, db: &ShimsDB) -> Result<()> {
        let serialized = bincode::serialize(db)?;
        fs::write(self.path, &serialized)?;
        info!("Successfully saved db");
        Ok(())
    }

    /// Returns the full path to the shimmed executable.
    pub fn get_full_executable_path(
        &self,
        exe: &str,
        tool: &str,
        version: &str,
    ) -> Result<Option<PathBuf>> {
        let root = self
            .tools_install_dir
            .to_str()
            .ok_or(anyhow!("Couldn't parse install dir as string."))?;
        let path: PathBuf = [&root, tool, version, "bin", exe].iter().collect();
        Ok(if path.exists() { Some(path) } else { None })
    }

    /// Resolve executable name as shim even if entered without extension.
    pub fn resolve_command(&self, exe: &str) -> Result<Option<String>> {
        for entry in fs::read_dir(&self.shims_dir)? {
            let name = entry?
                .file_name()
                .into_string()
                .map_err(|e| anyhow!("could not convert {:?} to string", e))?;
            if exe == &name {
                return Ok(Some(name));
            }
            for ext in EXTENSIONS.iter() {
                let with_ext = format!("{}.{}", exe, ext);
                if with_ext == name {
                    return Ok(Some(name));
                }
            }
        }
        // Fix: test!
        Ok(None)
    }

    /// Find a plugin which owns this exe
    pub fn find_plugin(&self, exe: &str) -> Result<Option<String>> {
        let shims = self.load_db()?;
        Ok(shims.get(exe).map(|s| s.to_string()))
    }

    /// Generates all required shims. Cleans up the shims directory before if desired.
    pub fn create_shims(&self, cleanup: bool) -> Result<()> {
        if cleanup {
            debug!("resetting shims directory");
            fs::remove_dir_all(self.shims_dir).context("cleaning up shims directory")?;
            fs::create_dir(self.shims_dir).context("recreating shims directory after cleanup")?;
        }
        let db = self.load_db()?;
        for exe in db.keys() {
            let target = self.shims_dir.join(&exe);
            debug!("Creating shim for {}", &exe);
            fs::copy(&self.shim_exe, target).context(format!("creating shim for {}", &exe))?;
        }
        Ok(())
    }

    pub fn generate_db_from_installed_tools(&self) -> Result<ShimsDB> {
        let mut db: ShimsDB = HashMap::new();

        for entry in fs::read_dir(self.tools_install_dir)? {
            let entry = entry?;
            let tool = entry.file_name().into_string().unwrap(); // Can we trust NTFS to always have unicode filenames?
            for version in fs::read_dir(entry.path())? {
                let version = version?;
                if version.path().is_dir() {
                    let mut path = version.path();
                    path.push("bin");
                    for exe in fs::read_dir(path)? {
                        let exe = exe?;
                        if valid_exe_extension(exe.path().extension()) {
                            let exe_name = exe.file_name().into_string().unwrap();
                            let old_value = db.insert(exe_name.clone(), tool.clone());
                            if let Some(value) = old_value {
                                if value != tool {
                                    return Err(anyhow!(
                                        "{} appears in two tools: {} and {}",
                                        &exe_name,
                                        &tool,
                                        &value
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(db)
    }
}

fn valid_exe_extension(extension: Option<&OsStr>) -> bool {
    for item in EXTENSIONS.iter() {
        if Some(OsStr::new(item)) == extension {
            return true;
        }
    }
    return false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::ChildPath, prelude::*, TempDir};
    use rstest::rstest;
    use std::{fs::OpenOptions, str::FromStr};

    struct TestPaths {
        tools_install_dir: ChildPath,
        db_path: ChildPath,
        shims_dir: ChildPath,
        shim_exe: ChildPath,
    }

    fn test_paths(root: &TempDir) -> TestPaths {
        let tools_install_dir = root.child("installs");
        tools_install_dir.create_dir_all().unwrap();
        let shims_dir = root.child("shims");
        shims_dir.create_dir_all().unwrap();
        let db_path = root.child("shims.db");
        let shim_exe = root.child("shim.exe");
        shim_exe.touch().unwrap();
        TestPaths {
            tools_install_dir,
            db_path,
            shims_dir,
            shim_exe,
        }
    }

    fn test_data() -> ShimsDB {
        HashMap::from([
            ("kubectl.exe".to_string(), "kubectl".to_string()),
            ("docker.exe".to_string(), "docker".to_string()),
            ("minikube.exe".to_string(), "minikube".to_string()),
            ("kubectx.exe".to_string(), "kubectx".to_string()),
            ("kubens.exe".to_string(), "kubectx".to_string()),
        ])
    }

    #[test]
    fn save_and_load_shims_db() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        shims.save_db(&db).unwrap();
        let loaded = shims.load_db().unwrap();
        assert_eq!(db, loaded);
    }

    #[rustfmt::skip]
    #[rstest]
    #[case(vec!["hello.exe", "world.exe"], "hello.exe", Some("hello.exe".to_string()), "exact match")]
    #[case(vec!["hello.exe", "world.exe"], "hello", Some("hello.exe".to_string()), "missing extension")]
    #[case(vec!["hello.exe", "world.exe"], "what.exe", None, "invalid command")]
    fn resolve_command_tests(
        #[case] existing_shims: Vec<&str>,
        #[case] exe: &str,
        #[case] expected: Option<String>,
        #[case] msg: &str,
    ) {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        for n in existing_shims {
            paths.shims_dir.child(&n).touch().unwrap();
        }
        let result = shims.resolve_command(&exe).unwrap();
        assert_eq!(result, expected, "test case: {}", &msg);
    }

    #[test]
    fn find_plugin_with_existing_plugin_returns_valid_plugin() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        shims.save_db(&db).unwrap();
        let result = shims.find_plugin("kubens.exe").unwrap();
        assert_eq!(result, Some("kubectx".to_string()));
    }

    #[test]
    fn find_plugin_with_invalid_plugin_returns_none() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        shims.save_db(&db).unwrap();
        let result = shims.find_plugin("mycmd").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_should_succeed() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
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

        let db = test_data();
        let generated = shims.generate_db_from_installed_tools().unwrap();
        assert_eq!(db, generated);
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_should_not_include_files_without_valid_extension() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.2.4").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").create_dir_all().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").child("kubectl.exe").touch().unwrap();
        paths.tools_install_dir.child("kubectl").child("1.1").child("bin").child("kubectl.txt").touch().unwrap();
        let generated = shims.generate_db_from_installed_tools().unwrap();
        assert!(!generated.contains_key("kubectl.txt"), "should not contain files with wrong extension");
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_with_same_executable_name_in_two_tools_should_fail() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
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
    fn test_get_full_executable_path_when_version_does_not_exist_returns_none() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        #[rustfmt::skip]
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        let tool = "mytool";
        let exe = "myexe";
        let version = "v1.0.1";
        let result = shims.get_full_executable_path(exe, tool, version);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_full_executable_path_when_version_exists_returns_path() {
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        let tool = "mytool";
        let exe = "myexe";
        let version = "v1.0.1";
        let binary = paths.tools_install_dir.child(&tool).child(&version).child("bin").child(&exe);
        binary.touch().unwrap();
        let path = binary.to_str().unwrap();
        let result = shims.get_full_executable_path(exe, tool, version);
        assert_eq!(result.unwrap(), Some(PathBuf::from_str(path).unwrap()));
    }

    #[test]
    #[rustfmt::skip]
    fn create_shims_without_cleanup_should_create_shims_that_exists_in_the_db() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        shims.save_db(&db).unwrap();
        shims.create_shims(false).unwrap();
        db.keys().for_each(|k| {
            assert!(shims.shims_dir.join(k).exists(), "shim '{}' does not exist", &k);
        });
        assert_eq!(shims.shims_dir.read_dir().unwrap().count(), 5);
    }

    #[test]
    #[rustfmt::skip]
    fn create_shims_without_cleanup_leaves_dangling_shims_in_place() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let paths = test_paths(&tmp_dir);
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
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
        let shims = Shims::new(&paths.db_path, &paths.tools_install_dir, &paths.shims_dir, &paths.shim_exe).unwrap();
        let dangling = shims.shims_dir.join("invalid.exe");
        OpenOptions::new().create(true).write(true).open(&dangling).unwrap();
        shims.save_db(&db).unwrap();
        shims.create_shims(true).unwrap();
        assert!(!dangling.exists(), "dangling file was not deleted in create shims with cleanup");
        assert_eq!(shims.shims_dir.read_dir().unwrap().count(), 5);
    }
}
