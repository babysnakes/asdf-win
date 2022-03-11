use asdfw::{
    plugins::plugin_manager::PluginManager,
    runtime::RuntimeEnvironment,
    shims::{Shims, ShimsDB},
};
use assert_fs::{fixture::ChildPath, prelude::*, TempDir};

pub struct Paths {
    pub current_dir: ChildPath,
    pub home_dir: ChildPath,
    pub shims_db: ChildPath,
    pub installs_dir: ChildPath,
    pub shims_dir: ChildPath,
    pub shim_exe: ChildPath,
    pub plugins_dir: ChildPath,
    pub log_dir: ChildPath,
    pub global_tool_versions_file: ChildPath,
}

impl Paths {
    pub fn new<'a>(root: &'a TempDir, global: &str, local: Option<&str>) -> Paths {
        let home_dir = root.child("home");
        home_dir.create_dir_all().unwrap();
        let global_tool_versions_file = home_dir.child(".tool-versions");
        global_tool_versions_file.write_str(global).unwrap();
        let current_dir = root.child("current_dir");
        current_dir.create_dir_all().unwrap();
        if let Some(text) = local {
            let lf = current_dir.child(".tool-versions");
            lf.write_str(text).unwrap();
        }
        let installs_dir = root.child("installs");
        installs_dir.create_dir_all().unwrap();
        let shims_dir = root.child("shims");
        shims_dir.create_dir_all().unwrap();
        let log_dir = root.child("logs");
        log_dir.create_dir_all().unwrap();
        let shim_exe = root.child("shim.exe");
        shim_exe.touch().unwrap();
        let shims_db = root.child("shims.db");
        let plugins_dir = root.child("plugins");
        Paths {
            current_dir,
            home_dir,
            installs_dir,
            shims_db,
            shims_dir,
            log_dir,
            shim_exe,
            plugins_dir,
            global_tool_versions_file,
        }
    }

    pub fn to_environment(&self) -> RuntimeEnvironment {
        RuntimeEnvironment {
            current_dir: self.current_dir.to_path_buf(),
            home_dir: self.home_dir.to_path_buf(),
            installs_dir: self.installs_dir.to_path_buf(),
            shims_db: self.shims_db.to_path_buf(),
            shims_dir: self.shims_dir.to_path_buf(),
            log_dir: self.log_dir.to_path_buf(),
            shim_exe: self.shim_exe.to_path_buf(),
            plugins_dir: self.plugins_dir.to_path_buf(),
            global_tool_versions_file: self.global_tool_versions_file.to_path_buf(),
        }
    }

    pub fn generate_shims_db(&self) -> ShimsDB {
        let pm = PluginManager::new(&self.plugins_dir);
        let shims = Shims::new(&self.shims_db, &self.installs_dir, &self.shims_dir, &self.shim_exe, pm).unwrap();
        let db = shims.generate_db_from_installed_tools().unwrap();
        shims.save_db(&db).unwrap();
        db
    }
}

#[rustfmt::skip]
pub fn fixture_installed_tools(dir: &'_ ChildPath) {
    dir.child("mytool1").child("1.2.4").child("bin").create_dir_all().unwrap();
    dir.child("mytool1").child("1.2.4").child("bin").child("cmd1.exe").touch().unwrap();
    dir.child("mytool1").child("1.1").child("bin").create_dir_all().unwrap();
    dir.child("mytool1").child("1.1").child("bin").child("cmd1.exe").touch().unwrap();
    dir.child("mytool2").child("v1.17").child("bin").create_dir_all().unwrap();
    dir.child("mytool2").child("v1.17").child("bin").child("cmd2.exe").touch().unwrap();
    dir.child("mytool2").child("v1.19").child("bin").create_dir_all().unwrap();
    dir.child("mytool2").child("v1.19").child("bin").child("cmd2.exe").touch().unwrap();
    dir.child("mytool3").child("2.5").child("bin").create_dir_all().unwrap();
    dir.child("mytool3").child("2.5").child("bin").child("cmd3.exe").touch().unwrap();
    dir.child("mytool4").child("0.12").child("bin").create_dir_all().unwrap();
    dir.child("mytool4").child("0.12").child("bin").child("cmd4.exe").touch().unwrap();
    dir.child("mytool4").child("0.12").child("bin").child("cmd5.exe").touch().unwrap();
}

pub fn test_data_matching_shims<'a>(shims_dir: &'a ChildPath, db: &'a ShimsDB) {
    db.keys().for_each(|k| shims_dir.child(k).touch().unwrap())
}
