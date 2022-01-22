use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub type ShimsDB = HashMap<String, String>;

/// The Shims struct contains data required for handling shims.
pub struct Shims<'a> {
    path: &'a Path,
    tools_install_dir: &'a Path,
}

impl<'a> Shims<'a> {
    /// Create a new Shims struct from the provided db path and installations
    /// directory.
    pub fn new(db_path: &'a Path, tools_install_dir: &'a Path) -> Result<Self> {
        if !tools_install_dir.is_dir() {
            return Err(anyhow!(
                "Supplied tools install dir ({:?}) is not an existing directory",
                tools_install_dir
            ));
        };
        Ok(Shims {
            path: db_path,
            tools_install_dir,
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
        Ok(())
    }

    /// Returns the full path to the shimmed executable.
    pub fn get_full_executable_path(
        &self,
        exe: &str,
        tool: &str,
        version: &str,
    ) -> Result<PathBuf> {
        let root = self
            .tools_install_dir
            .to_str()
            .ok_or(anyhow!("Couldn't parse install dir as string."))?;
        Ok([&root, tool, version, "bin", exe].iter().collect())
    }

    /// Find a plugin which owns this exe
    pub fn find_plugin(&self, exe: &str) -> Result<Option<String>> {
        let shims = self.load_db()?;
        Ok(shims.get(exe).map(|s| s.to_string()))
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

fn valid_exe_extension(extention: Option<&OsStr>) -> bool {
    Some(OsStr::new("exe")) == extention
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{prelude::*, TempDir};
    use std::str::FromStr;

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
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path(), tmp_dir.path()).unwrap();
        shims.save_db(&db).unwrap();
        let loaded = shims.load_db().unwrap();
        assert_eq!(db, loaded);
    }

    #[test]
    fn find_plugin_with_existing_plugin_returns_valid_plugin() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path(), tmp_dir.path()).unwrap();
        shims.save_db(&db).unwrap();
        let result = shims.find_plugin("kubens.exe").unwrap();
        assert_eq!(result, Some("kubectx".to_string()));
    }

    #[test]
    fn find_plugin_with_invalid_plugin_returns_none() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path(), tmp_dir.path()).unwrap();
        shims.save_db(&db).unwrap();
        let result = shims.find_plugin("mycmd").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_should_succeed() {
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let installs_dir = tmp_dir.child("installs");
        installs_dir.child("kubectl").child("1.2.4").child("bin").create_dir_all().unwrap();
        installs_dir.child("kubectl").child("1.2.4").child("bin").child("kubectl.exe").touch().unwrap();
        installs_dir.child("kubectl").child("1.1").child("bin").create_dir_all().unwrap();
        installs_dir.child("kubectl").child("1.1").child("bin").child("kubectl.exe").touch().unwrap();
        installs_dir.child("docker").child("v1.17").child("bin").create_dir_all().unwrap();
        installs_dir.child("docker").child("v1.17").child("bin").child("docker.exe").touch().unwrap();
        installs_dir.child("docker").child("v1.19").child("bin").create_dir_all().unwrap();
        installs_dir.child("docker").child("v1.19").child("bin").child("docker.exe").touch().unwrap();
        installs_dir.child("minikube").child("2.5").child("bin").create_dir_all().unwrap();
        installs_dir.child("minikube").child("2.5").child("bin").child("minikube.exe").touch().unwrap();
        installs_dir.child("kubectx").child("0.12").child("bin").create_dir_all().unwrap();
        installs_dir.child("kubectx").child("0.12").child("bin").child("kubectx.exe").touch().unwrap();
        installs_dir.child("kubectx").child("0.12").child("bin").child("kubens.exe").touch().unwrap();

        let shims = Shims::new(&db_file, &installs_dir).unwrap();
        let db = test_data();
        let generated = shims.generate_db_from_installed_tools().unwrap();
        assert_eq!(db, generated);
    }

    #[test]
    #[rustfmt::skip]
    fn generate_shims_with_same_executable_name_in_two_tools_should_fail() {
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let installs_dir = tmp_dir.child("installs");
        installs_dir.child("kubectl").child("1.2.4").child("bin").create_dir_all().unwrap();
        // !! The executable created below should trigger an error:
        installs_dir.child("kubectl").child("1.2.4").child("bin").child("kubens.exe").touch().unwrap();
        installs_dir.child("kubectl").child("1.1").child("bin").create_dir_all().unwrap();
        installs_dir.child("kubectl").child("1.1").child("bin").child("kubectl.exe").touch().unwrap();
        installs_dir.child("kubectx").child("0.12").child("bin").create_dir_all().unwrap();
        installs_dir.child("kubectx").child("0.12").child("bin").child("kubectx.exe").touch().unwrap();
        installs_dir.child("kubectx").child("0.12").child("bin").child("kubens.exe").touch().unwrap();

        let shims = Shims::new(&db_file, &installs_dir).unwrap();
        let res = shims.generate_db_from_installed_tools();
        if let Err(e) = res {
            let err = format!("{:?}", e);
            assert!(err.contains("kubens.exe"), "Wrong error was triggered ({:?}), should contain 'kubens.exe'", err);
        } else {
            assert!(false, "Same executable name in different tools should have triggered error");
        }
    }

    #[test]
    fn test_get_full_executable_path() {
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path(), tmp_dir.path()).unwrap();
        let tool = "mytool";
        let exe = "myexe";
        let version = "v1.0.1";
        let path = format!(
            "{}/{}/{}/bin/{}",
            tmp_dir.to_str().unwrap(),
            tool,
            version,
            exe
        );
        let result = shims.get_full_executable_path(exe, tool, version);
        assert_eq!(result.unwrap(), PathBuf::from_str(&path).unwrap());
    }
}
