use anyhow::{anyhow, Result};
use std::{env, path::PathBuf};

const CUSTOM_APP_DIR_ENV: &str = "ASDFW_CUSTOM_APPDIR";

#[derive(Debug)]
pub struct RuntimeEnvironment {
    pub current_dir: PathBuf,
    pub home_dir: PathBuf,
    pub shims_db: PathBuf,
    pub installs_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub shim_exe: PathBuf,
    pub plugins_dir: PathBuf,
    pub log_dir: PathBuf,
    pub global_tool_versions_file: PathBuf,
}

impl RuntimeEnvironment {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or(anyhow!("Could not get home directory"))?;
        let current_dir = std::env::current_dir()?;
        let app_dir = match env::var(CUSTOM_APP_DIR_ENV) {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => home_dir.join(".asdfw"),
        };
        let shims_db = app_dir.join("shims.db");
        let installs_dir = app_dir.join("installs");
        let shims_dir = app_dir.join("shims");
        let shim_exe = app_dir.join("lib").join("shim.exe");
        let plugins_dir = app_dir.join("plugins");
        let log_dir = app_dir.join("logs");
        let global_tool_versions_file = home_dir.join(".tool-versions");
        Ok(RuntimeEnvironment {
            home_dir,
            current_dir,
            shims_db,
            installs_dir,
            shims_dir,
            shim_exe,
            plugins_dir,
            log_dir,
            global_tool_versions_file,
        })
    }
}
