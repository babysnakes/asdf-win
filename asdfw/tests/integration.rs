mod common;

use asdfw::common::*;
use assert_fs::{prelude::*, TempDir};
use common::Paths;
use rstest::rstest;

#[test]
fn which_with_no_tool_configured_should_return_error_no_tool() {
    let versions = "mytool1 1.2.4";
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, versions, None);
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let err = find_path_for_cmd(&env, "no-cmd.exe").unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("No tool configured"),
        "Wrong error message for no tool configured: {}",
        &msg
    );
}

#[test]
fn which_with_no_version_configured_should_return_error_no_version() {
    let versions = "mytool1 1.2.4";
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, versions, None);
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let err = find_path_for_cmd(&env, "cmd3.exe").unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("No version"), "wrong error message for no configured version: {}", &msg);
}

#[rstest]
#[case("mytool5", "1.0", "mycmd.exe", "mycmd.exe", "searching with full name")]
#[case("mytool5", "1.0", "mycmd", "mycmd.exe", "missing '.exe' extension")]
fn which_should_return_valid_executable_path(
    #[case] tool: &str,
    #[case] version: &str,
    #[case] search_exe: &str,
    #[case] create_exe: &str,
    #[case] msg: &str,
) {
    let versions = format!("{} {}", &tool, &version);
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, &versions, None);
    let mytool_dir = paths.installs_dir.child(&tool).child(&version).child("bin");
    mytool_dir.create_dir_all().unwrap();
    mytool_dir.child(&create_exe).touch().unwrap();
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let expected = mytool_dir.child(&create_exe).path().to_string_lossy().into_owned();
    let result = find_path_for_cmd(&env, &search_exe).unwrap();
    assert_eq!(expected, result, "wrong path from `which`. case: {}", msg);
}
