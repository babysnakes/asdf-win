use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct RuntimeEnvironment {
    pub current_dir: PathBuf,
    pub home_dir: PathBuf,
    pub shims_db: PathBuf,
    pub installs_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl RuntimeEnvironment {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or(anyhow!("Could not get home directory"))?;
        let current_dir = std::env::current_dir()?;
        let app_dir = home_dir.join(".asdfw");
        let shims_db = app_dir.join("shims.db");
        let installs_dir = app_dir.join("installs");
        let log_dir = app_dir.join("logs");
        Ok(RuntimeEnvironment {
            home_dir,
            current_dir,
            shims_db,
            installs_dir,
            log_dir,
        })
    }
}
