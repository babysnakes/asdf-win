use anyhow::Result;
use std::{ffi::OsStr, path::Path};

use super::plugin::Plugin;

pub struct PluginManager<'a> {
    plugins_path: &'a Path,
}

impl<'a> PluginManager<'a> {
    pub fn new(plugins_path: &'a Path) -> Self {
        PluginManager { plugins_path }
    }

    pub fn get_plugin(&self, name: &'a OsStr) -> Result<Plugin> {
        Plugin::new(name, self.plugins_path.join(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    #[test]
    fn get_plugin_returns_plugin_with_provided_name() {
        let tool = OsStr::new("mytool");
        let plugins_dir = TempDir::new().unwrap().to_path_buf();
        let pm = PluginManager {
            plugins_path: &plugins_dir,
        };
        let result = pm.get_plugin(tool).unwrap();
        assert_eq!(result.name, tool);
    }

    #[test]
    fn get_plugin_returns_plugin_with_directory_matching_the_plugin_name() {
        let tool = OsStr::new("mytool");
        let plugins_dir = TempDir::new().unwrap().to_path_buf();
        let pm = PluginManager {
            plugins_path: &plugins_dir,
        };
        let result = pm.get_plugin(tool).unwrap();
        let expected_path = plugins_dir.as_path().join(tool);
        assert_eq!(expected_path, result.dir);
    }
}
