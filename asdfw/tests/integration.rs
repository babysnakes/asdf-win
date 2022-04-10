mod common;

use asdfw::common::*;
use assert_fs::{prelude::*, TempDir};
use common::Paths;
use rstest::rstest;

#[test]
fn which_with_no_tool_exists_should_return_error_no_tool() {
    let versions = "mytool1 1.2.4";
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, versions, None);
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let shim = paths.shims_dir.child("no-cmd.exe");
    shim.touch().unwrap(); // make sure it exists as a shim - otherwise this is different error
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

#[test]
fn which_with_invalid_command_should_return_matching_error() {
    let cmd_name = "nosuchshim";
    let versions = "mytool1 1.2.4";
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, versions, None);
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let err = find_path_for_cmd(&env, cmd_name).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("Could not find shim"),
        "should explain that shim does not exist, got: '{msg}'"
    );
    assert!(msg.contains(cmd_name), "should repeat shim name. got: '{msg}'");
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
    let expected = mytool_dir.child(&create_exe).as_os_str().to_owned();
    let result = find_path_for_cmd(&env, search_exe).unwrap();
    assert_eq!(expected, result, "wrong path from `which`. case: {}", msg);
}

#[test]
fn which_with_version_configured_but_not_installed_should_return_matching_error() {
    let tool = "mytool5";
    let version = "1.2.5";
    let cmd = "cmd10.exe";
    let alternate_version = "1.2.4";
    let versions = format!("{} {}", &tool, &version);
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, &versions, None);
    let mytool_alternate_version_dir = paths.installs_dir.child(&tool).child(&alternate_version).child("bin");
    mytool_alternate_version_dir.create_dir_all().unwrap();
    mytool_alternate_version_dir.child(&cmd).touch().unwrap();
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let result = find_path_for_cmd(&env, cmd);
    assert!(result.is_err(), "missing version should return error");
    let err = result.unwrap_err();
    let msg = format!("{err}");
    let expected_msg = "configured but not installed";
    assert!(
        msg.contains(expected_msg),
        "Expected error to contain '{expected_msg}', got: '{msg}'"
    );
    assert!(msg.contains(version), "Expected error to reference configured version, got: '{msg}'");
}

#[test]
fn which_with_specific_command_missing_should_return_matching_error() {
    let tool = "mytool5";
    let version = "1.2.5";
    let cmd1 = "cmd11.exe";
    let cmd2 = "cmd12.exe";
    let alternate_version = "1.2.4";
    let versions = format!("{} {}", &tool, &version);
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, &versions, None);
    let mytool_dir = paths.installs_dir.child(&tool).child(&version).child("bin");
    mytool_dir.create_dir_all().unwrap();
    // cmd1 is not installed in `version`!
    mytool_dir.child(&cmd2).touch().unwrap();
    let mytool_alternate_version_dir = paths.installs_dir.child(&tool).child(&alternate_version).child("bin");
    mytool_alternate_version_dir.create_dir_all().unwrap();
    mytool_alternate_version_dir.child(&cmd1).touch().unwrap();
    mytool_alternate_version_dir.child(&cmd2).touch().unwrap();
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let result = find_path_for_cmd(&env, cmd1);
    assert!(result.is_err(), "missing version should return error");
    let err = result.unwrap_err();
    let msg = format!("{err}");
    let expected_msg = "does not exist in version";
    assert!(
        msg.contains(expected_msg),
        "Expected error to contain '{expected_msg}', got: '{msg}'"
    );
    assert!(msg.contains(version), "Expected error to reference configured version, got: '{msg}'");
}

#[test]
fn which_with_specific_tool_does_not_check_for_tool_existance() {
    // these are details of a tool that is not installed
    let tool = "nosuchtool";
    let version = "1.23.4";
    let cmd = "nocmd.exe";
    let versions = format!("{} {}", &tool, &version);
    let tmp_dir = TempDir::new().unwrap();
    let paths = Paths::new(&tmp_dir, &versions, None);
    let env = paths.to_environment();
    common::fixture_installed_tools(&paths.installs_dir);
    let db = paths.generate_shims_db();
    common::test_data_matching_shims(&paths.shims_dir, &db);
    let error = find_path_for_cmd_with_tool(&env, cmd, tool).unwrap_err();
    let message = format!("{}", error);
    assert!(message.contains("Could not find shim"), "error should complain about finding shim");
    assert!(message.contains(cmd), "error should specify the command");
}
