use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub type ShimsDB = HashMap<String, String>;

pub struct Shims<'a> {
    pub path: &'a Path,
}

impl<'a> Shims<'a> {
    pub fn new(path: &'a Path) -> Self {
        Shims { path }
    }

    fn load_db(&self) -> Result<ShimsDB> {
        let contents = fs::read(self.path)?;
        bincode::deserialize(&contents)
            .map_err(|err| anyhow!("Error deserializing ShimsDB: {}", err))
    }

    pub fn save_db(&self, db: &ShimsDB) -> Result<()> {
        let serialized = bincode::serialize(db)?;
        fs::write(self.path, &serialized)?;
        Ok(())
    }

    // Find a plugin which owns this exe
    pub fn find_plugin(&self, exe: &str) -> Result<Option<String>> {
        let shims = self.load_db()?;
        Ok(shims.get(exe).map(|s| s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::PathChild, TempDir};

    fn test_data() -> ShimsDB {
        HashMap::from([
            ("kubectl".to_string(), "kubectl".to_string()),
            ("docker".to_string(), "docker".to_string()),
            ("minikube".to_string(), "minikube".to_string()),
            ("kubectx".to_string(), "kubectx".to_string()),
            ("kubens".to_string(), "kubectx".to_string()),
        ])
    }

    #[test]
    fn save_and_load_shims_db() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path());
        shims.save_db(&db).unwrap();
        let loaded = shims.load_db().unwrap();
        assert_eq!(db, loaded);
    }

    #[test]
    fn find_plugin_with_existing_plugin_returns_valid_plugin() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path());
        shims.save_db(&db).unwrap();
        let result = shims.find_plugin("kubens").unwrap();
        assert_eq!(result, Some("kubectx".to_string()));
    }

    #[test]
    fn find_plugin_with_invalid_plugin_returns_none() {
        let db = test_data();
        let tmp_dir = TempDir::new().unwrap();
        let db_file = tmp_dir.child("shims.db");
        let shims = Shims::new(db_file.path());
        shims.save_db(&db).unwrap();
        let result = shims.find_plugin("mycmd").unwrap();
        assert_eq!(result, None);
    }
}
