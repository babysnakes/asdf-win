use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_DIR: &str = "bin";
pub const PLUGIN_FILENAME: &str = "plugin.yaml";

#[derive(Debug)]
pub struct Plugin<'a> {
    pub name: &'a str,
    pub dir: PathBuf,
    pub config: PluginConfig,
}

#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "PluginConfig::default_bin_dirs")]
    pub bin_dirs: Vec<String>,
    #[serde(default)]
    pub env_vars: Vec<EnvVar>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde()]
pub struct EnvVar {
    name: String,
    value: EnvVarValue,
    #[serde(default)]
    overriding_name: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EnvVarValue {
    Value { value: String },
    RelativeInstallPath { relative_inst_path: String },
}

impl PluginConfig {
    fn default_bin_dirs() -> Vec<String> {
        vec![DEFAULT_DIR.to_string()]
    }
}

impl<'a> Plugin<'a> {
    pub fn new(name: &'a str, path: PathBuf) -> Result<Self> {
        let plugin_file = path.as_path().join(PLUGIN_FILENAME);
        let config = if plugin_file.exists() {
            let f = File::open(plugin_file).with_context(|| format!("opening plugin config file for '{name}'"))?;
            let config: PluginConfig =
                serde_yaml::from_reader(f).with_context(|| format!("Parsing plugin config for '{name}'"))?;
            if config.bin_dirs.is_empty() {
                PluginConfig {
                    bin_dirs: PluginConfig::default_bin_dirs(),
                    ..config
                }
            } else {
                config
            }
        } else {
            PluginConfig {
                bin_dirs: PluginConfig::default_bin_dirs(),
                env_vars: vec![],
            }
        };
        Ok(Plugin {
            name,
            config,
            dir: path,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_fs::{prelude::*, TempDir};
    use rstest::rstest;
    use stripmargin::StripMargin;

    #[test]
    fn plugin_new_with_config_should_return_parsed_config() {
        let tool = "mytool";
        let yaml = "---\nbin_dirs:\n  - some/dir\n  - otherdir";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let plugin = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        assert_eq!(plugin.name, tool);
        assert_eq!(plugin.config.bin_dirs, vec!["some/dir".to_string(), "otherdir".to_string()]);
    }

    #[test]
    fn plugin_new_without_config_should_return_config_default_values() {
        let tool = "mytool";
        let tmpdir = TempDir::new().unwrap();
        let plugin_dir = tmpdir.child(tool);
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        assert_eq!(result.name, tool);
        assert_eq!(
            result.config.bin_dirs,
            PluginConfig::default_bin_dirs(),
            "expected config to be None, got: {:?}",
            result.config
        );
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
    fn plugin_should_return_provided_bin_directories() {
        let tool = "mytool";
        let yaml = "---\nbin_dirs:\n  - some/dir\n  - otherdir";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected = vec!["some/dir", "otherdir"];
        assert_eq!(result.config.bin_dirs, expected);
    }

    #[test]
    fn plugin_should_return_default_bin_directories_if_no_config_file_exists() {
        let tool = "mytool";
        let tmpdir = TempDir::new().unwrap();
        let plugin_dir = tmpdir.child(tool);
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected = vec!["bin"];
        assert_eq!(result.config.bin_dirs, expected);
    }

    #[rstest]
    #[case("---\nbin_dirs: []\n", "explicit empty bin dirs")]
    #[case("---\nsome_list: []\n", "absent bin dirs")]
    fn plugin_should_return_default_bin_directories_if_empty_in_config(#[case] yml: &str, #[case] msg: &str) {
        let tool = "mytool";
        let yaml = yml;
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected = vec!["bin"];
        assert_eq!(result.config.bin_dirs, expected, "expected default config (test case: {})", &msg);
    }

    #[rstest]
    #[case(
        r#"---
          |bin_dirs:
          |  - some/dir
          |  - otherdir
          |env_vars:
          |  - name: MYENV
          |    value:
          |      value: "A Value"
          |"#
          .strip_margin(),
          vec![
            EnvVar {
                name: "MYENV".to_string(),
                overriding_name: None,
                value: EnvVarValue::Value { value: "A Value".to_string() }
            },
          ],
          "value env"
    )]
    #[case(
        r#"---
          |env_vars:
          |  - name: MYENV
          |    value:
          |      relative_inst_path: "relative\\path"
          |"#
          .strip_margin(),
          vec![
            EnvVar {
                name: "MYENV".to_string(),
                overriding_name: None,
                value: EnvVarValue::RelativeInstallPath { relative_inst_path: "relative\\path".to_string() }
            },
          ],
          "relative path env"
    )]
    #[case(
        r#"---
          |env_vars:
          |  - name: MYENV
          |    value:
          |      value: "A Value"
          |    overriding_name: MYENV_OVERRIDE
          |"#
          .strip_margin(),
          vec![
            EnvVar {
                name: "MYENV".to_string(),
                overriding_name: Some("MYENV_OVERRIDE".to_string()),
                value: EnvVarValue::Value { value: "A Value".to_string() }
            },
          ],
          "A value with override"
    )]
    fn plugin_returns_provided_environment_variables(
        #[case] yaml: String,
        #[case] expected: Vec<EnvVar>,
        #[case] msg: &str,
    ) {
        let tool = "mytool";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        plugin_yaml.write_str(&yaml).unwrap();
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        assert_eq!(result.config.env_vars, expected, "test case: {msg}");
    }

    #[rstest]
    #[case(Some("---\nbin_dirs: []\n"), "config without reference to env vars")]
    #[case(None, "no config file")]
    fn plugin_without_provided_environment_variables_returns_none(#[case] yml: Option<&str>, #[case] msg: &str) {
        let tool = "mytool";
        let plugin_dir = TempDir::new().unwrap();
        let plugin_yaml = plugin_dir.child(PLUGIN_FILENAME);
        if let Some(txt) = yml {
            plugin_yaml.write_str(txt).unwrap();
        }
        let result = Plugin::new(tool, plugin_dir.to_path_buf()).unwrap();
        let expected: Vec<EnvVar> = vec![];
        assert_eq!(result.config.env_vars, expected, "expected default env vars (test case: {})", &msg);
    }
}
