use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_DIR: &str = "bin";
pub const PLUGIN_FILENAME: &str = "plugin.yaml";

#[derive(Debug)]
pub struct Plugin<'a> {
    pub name: &'a str,
    pub dir: PathBuf,
    config: Option<PluginConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    #[serde(default)]
    bin_dirs: Option<Vec<String>>,
}

impl<'a> Plugin<'a> {
    pub fn new(name: &'a str, path: PathBuf) -> Result<Self> {
        let plugin_file = path.as_path().join(PLUGIN_FILENAME);
        let config = if plugin_file.exists() {
            let f = File::open(plugin_file).with_context(|| format!("opening plugin config file for '{name}'"))?;
            let config: PluginConfig =
                serde_yaml::from_reader(f).with_context(|| format!("Parsing plugin config for '{name}'"))?;
            Some(config)
        } else {
            None
        };
        Ok(Plugin {
            name,
            config,
            dir: path,
        })
    }

    pub fn get_bin_directories(&self) -> Vec<String> {
        match &self.config {
            None => vec![DEFAULT_DIR.to_string()],
            Some(config) => {
                let dirs = config.bin_dirs.clone().unwrap_or_default();
                if dirs.is_empty() {
                    vec![DEFAULT_DIR.to_string()]
                } else {
                    dirs
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_fs::{prelude::*, TempDir};

    #[test]
    fn plugin_new_with_config_should_return_some_config() {
        let tool = "mytool";
        let yaml = "---\nbin_dirs:\n  - some/dir\n  - otherdir";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        assert_eq!(result.name, tool);
        assert!(result.config.is_some(), "expected some config, received None");
        assert_eq!(
            result.config.unwrap().bin_dirs.unwrap(),
            vec!["some/dir".to_string(), "otherdir".to_string()]
        );
    }

    #[test]
    fn plugin_new_without_config_should_return_none() {
        let tool = "mytool";
        let tmpdir = TempDir::new().unwrap();
        let plugin_dir = tmpdir.child(tool);
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        assert_eq!(result.name, tool);
        assert!(result.config.is_none(), "expected config to be None, got: {:?}", result.config);
    }

    #[test]
    fn plugin_new_with_invalid_config_should_return_error() {
        let tool = "mytool";
        let yaml = "-someinvaliddata";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf());
        dbg!(&result);
        assert!(result.is_err(), "Invalid config yaml should have produced an error");
    }

    #[test]
    fn plugin_get_bin_directories_returns_bin_directories_if_supplied() {
        let tool = "mytool";
        let yaml = "---\nbin_dirs:\n  - some/dir\n  - otherdir";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected = vec!["some/dir", "otherdir"];
        assert_eq!(result.get_bin_directories(), expected);
    }

    #[test]
    fn plugin_get_bin_directories_return_default_bin_directories_if_no_config_exists() {
        let tool = "mytool";
        let tmpdir = TempDir::new().unwrap();
        let plugin_dir = tmpdir.child(tool);
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected = vec!["bin"];
        assert_eq!(result.get_bin_directories(), expected);
    }

    #[test]
    fn plugin_get_bin_directories_return_default_bin_directories_if_empty_in_config() {
        let tool = "mytool";
        let yaml = "---\nbin_dirs:\n";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected = vec!["bin"];
        assert_eq!(result.get_bin_directories(), expected);
    }
}
