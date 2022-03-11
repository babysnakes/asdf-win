use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use log::{debug, info};

const FILE_NAME: &str = ".tool-versions";

type ToolVersionsData = HashMap<String, String>;

pub struct ToolVersions<'a> {
    tool: &'a str,
    global_path: &'a Path,
    current_dir: &'a Path,
}

/// Used to chain `<Result<Option<_>>` when you want to evaluate the next fn
/// if the provided option is `None`
macro_rules! eval_if_none {
    ($opt: expr, $func: expr) => {
        match $opt {
            None => $func,
            Some(_) => Ok($opt),
        }
    };
}

impl<'a> ToolVersions<'a> {
    pub fn new(global_path: &'a Path, current_dir: &'a Path, tool: &'a str) -> Self {
        ToolVersions {
            global_path,
            current_dir,
            tool,
        }
    }

    pub fn get_version(&self) -> Result<Option<String>> {
        self.get_version_from_env()
            .and_then(|r| eval_if_none!(r, self.get_version_from_current_dir()))
            .and_then(|r| eval_if_none!(r, self.get_version_from_global()))
    }

    pub fn save_local(&self, version: &'a str) -> Result<()> {
        info!("Setting local ({:?}) version for '{}': {}", self.current_dir, self.tool, &version);
        let context = format!("setting local version for {}: {}", self.tool, version);
        let local_file = self.current_dir.join(FILE_NAME);
        set_tool_version(&local_file, self.tool, version).context(context)
    }

    pub fn save_global(&self, version: &'a str) -> Result<()> {
        info!("Setting global version for '{}': {}", self.tool, &version);
        let context = format!("setting global version for {}: {}", self.tool, version);
        set_tool_version(self.global_path, self.tool, version).context(context)
    }

    fn get_version_from_env(&self) -> Result<Option<String>> {
        let env_name = env_var_name_for_tool(self.tool);
        Ok(std::env::var(&env_name).ok())
    }

    fn get_version_from_current_dir(&self) -> Result<Option<String>> {
        let mut path = PathBuf::from(self.current_dir);
        Ok(loop {
            path.push(FILE_NAME);
            if path.is_file() {
                if let Some(ver) = search_tool_in_file(self.tool, &path)? {
                    break Some(ver);
                }
            }

            if !(path.pop() && path.pop()) {
                break None;
            }
        })
    }

    fn get_version_from_global(&self) -> Result<Option<String>> {
        debug!("Searching for version in global file: {}", &self.tool);
        search_tool_in_file(self.tool, self.global_path).context("Parsing global tool versions file")
    }
}

fn set_tool_version<'a>(path: &'a Path, tool: &'a str, version: &'a str) -> Result<()> {
    debug!("reading current tool versions from {:?}", &path);
    let mut tool_versions = load_file(path)?;
    let previous = tool_versions.insert(tool.to_string(), version.to_string());
    if previous.is_none() {
        debug!("setting new version for {}", &tool);
    } else {
        debug!("Setting updated version for {}", &tool);
    }
    save_file(tool_versions, path)
}

fn load_file(path: &'_ Path) -> Result<ToolVersionsData> {
    if !path.exists() {
        info!("Tool versions file '{:?}' does not exist. Returning empty versions.", &path);
        return Ok(HashMap::new());
    }
    let mut data = HashMap::new();
    let context = format!("reading tool versions from {:?}", &path);
    let file = File::open(path).context(context.clone())?;
    let lines = io::BufReader::new(file).lines().collect::<Result<Vec<_>, _>>().context(context)?;
    for line in lines {
        let (tool, version) = parse_line(&line).map(|(k, v)| (k.to_owned(), v.to_owned()))?;
        data.insert(tool, version);
    }
    Ok(data)
}

fn save_file(data: ToolVersionsData, path: &'_ Path) -> Result<()> {
    let pairs = Vec::from_iter(data.iter());
    let mut strings = pairs.iter().map(|(k, v)| format!("{} {}", k, v)).collect::<Vec<String>>();
    strings.push("".to_owned());
    let content = strings.join("\r\n");
    fs::write(&path, content).context(format!("Saving tool versions to: {:?}", &path))
}

fn search_tool_in_file<'a>(search_for: &'a str, path: &'a Path) -> Result<Option<String>> {
    let file = File::open(path)?;
    let lines = io::BufReader::new(file).lines();
    for line in lines {
        let line = line?;
        let (tool, ver) = parse_line(&line)?;
        if tool == search_for {
            return Ok(Some(ver.to_owned()));
        }
    }
    Ok(None)
}

fn parse_line(line: &'_ str) -> Result<(&str, &str)> {
    let mk_error = || anyhow!("Invalid tools versions line: {}", &line);

    let result = line.split_once(' ').ok_or_else(mk_error);
    if let Ok((tool, ver)) = result {
        if tool.is_empty() || ver.is_empty() {
            return Err(mk_error());
        }
        if ver.contains(' ') {
            return Err(mk_error());
        }
    }
    result
}

fn env_var_name_for_tool(tool: &'_ str) -> String {
    format!("ASDFW_{}_VERSION", String::from(tool).to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{prelude::*, NamedTempFile, TempDir};
    use rstest::rstest;

    const SUBDIR: &str = "subdir";
    const FIXTURE_GLOBAL: &str = "tool1 v1.2\r\ntool2 v2.1.3\r\ntool3 v12\r\n";
    const FIXTURE_LOCAL: &str = "tool1 v1.3\r\ntool3 v10\r\n";
    const FIXTURE_LOCAL_SUBDIR: &str = "tool1 v1.4\r\n";
    const FIXTURE_TOOL1_GLOBAL: (&str, &str) = ("tool1", "v1.2");
    const _FIXTURE_TOOL2_GLOBAL: (&str, &str) = ("tool2", "v2.1.3");
    const _FIXTURE_TOOL3_GLOBAL: (&str, &str) = ("tool3", "v12");
    const FIXTURE_TOOL1_LOCAL: (&str, &str) = ("tool1", "v1.3");
    const FIXTURE_TOOL2_LOCAL: (&str, &str) = ("tool2", "v2.2.0");
    const FIXTURE_TOOL3_LOCAL: (&str, &str) = ("tool3", "v10");
    const FIXTURE_TOOL1_LOCAL_SUBDIR: (&str, &str) = ("tool1", "v1.4");

    fn gen_tool_versions_fixture() -> (NamedTempFile, TempDir) {
        let global_file = assert_fs::NamedTempFile::new(FILE_NAME).unwrap();
        global_file.write_str(FIXTURE_GLOBAL).unwrap();
        let current_dir = assert_fs::TempDir::new().unwrap();
        current_dir.child(FILE_NAME).write_str(FIXTURE_LOCAL).unwrap();
        let subdir = current_dir.child(SUBDIR);
        subdir.child(FILE_NAME).write_str(FIXTURE_LOCAL_SUBDIR).unwrap();
        (global_file, current_dir)
    }

    #[test]
    fn parse_line_returns_valid_values_on_valid_line() {
        let line = "my-tool v1.2.3";
        let (tool, ver) = parse_line(line).unwrap();
        assert_eq!(tool, "my-tool");
        assert_eq!(ver, "v1.2.3");
    }

    #[rstest]
    #[case("my-tool ", "missing version")]
    #[case("my-tool v1 1.2", "more then one space")]
    #[case("my-tool  v11.2", "two spaces separator")]
    fn parse_invalid_line(#[case] line: &str, #[case] msg: &str) {
        let res = parse_line(line);
        assert!(res.is_err(), "{} should return error", msg)
    }

    #[rstest]
    #[case("tool1 v1.2\r\ntool2 v2.1.3\r\ntool3 5.6\r\n", "tool3", "5.6")]
    fn find_version_in_file_existing_tool(#[case] content: &str, #[case] tool: &str, #[case] ver: String) {
        let temp_file = assert_fs::NamedTempFile::new(".tool_versions").unwrap();
        temp_file.write_str(content).unwrap();
        let res = search_tool_in_file(tool, temp_file.path()).unwrap();
        assert_eq!(res, Some(ver));
    }

    #[rstest]
    #[case("tool1 v1.2\r\ntool2 v2.1.3\r\ntool3 5.6\r\n", "tool4")]
    fn find_version_in_file_missing_tool(#[case] content: &str, #[case] tool: &str) {
        let temp_file = assert_fs::NamedTempFile::new(".tool_versions").unwrap();
        temp_file.write_str(content).unwrap();
        let res = search_tool_in_file(tool, temp_file.path()).unwrap();
        assert_eq!(res, None);
    }

    #[rstest]
    #[case("tool1 v1.2\r\ntool2 v2.1.3\r\ntool3  5.6\r\n", "tool3")]
    fn find_version_in_file_corrupt_file_if_reaches_corrupt_line(#[case] content: &str, #[case] tool: &str) {
        let temp_file = assert_fs::NamedTempFile::new(".tool_versions").unwrap();
        temp_file.write_str(content).unwrap();
        let res = search_tool_in_file(tool, temp_file.path());
        assert!(res.is_err(), "Corrupt file should produce error");
    }

    #[test]
    fn get_version_when_environment_variable_is_set() {
        let tool = "justfortest";
        let (global_file, current_dir) = gen_tool_versions_fixture();
        let tool_versions = ToolVersions::new(global_file.path(), current_dir.path(), tool);
        let custom_env = env_var_name_for_tool(tool);
        let expected = "1.1.1".to_string();
        std::env::set_var(&custom_env, &expected);
        let result = tool_versions.get_version().unwrap();
        std::env::remove_var(&custom_env);
        assert_eq!(Some(expected), result);
    }

    #[test]
    fn get_version_from_local_file() {
        let (global_file, current_dir) = gen_tool_versions_fixture();
        let (tool, ver) = FIXTURE_TOOL1_LOCAL;
        let tool_versions = ToolVersions::new(global_file.path(), current_dir.path(), tool);
        let result = tool_versions.get_version().unwrap();
        assert_eq!(result, Some(ver.to_string()));
    }

    #[test]
    fn get_version_from_nested_local_file() {
        let (global_file, current_dir) = gen_tool_versions_fixture();
        let (tool, ver) = FIXTURE_TOOL1_LOCAL_SUBDIR;
        let current_dir = current_dir.child(SUBDIR);
        let tool_versions = ToolVersions::new(global_file.path(), current_dir.path(), tool);
        let result = tool_versions.get_version().unwrap();
        assert_eq!(result, Some(ver.to_string()));
    }

    #[test]
    fn get_version_from_local_file_in_parent_dir() {
        let (global_file, current_dir) = gen_tool_versions_fixture();
        let (tool, ver) = FIXTURE_TOOL3_LOCAL;
        let current_dir = current_dir.child(SUBDIR);
        let tool_versions = ToolVersions::new(global_file.path(), current_dir.path(), tool);
        let result = tool_versions.get_version().unwrap();
        assert_eq!(result, Some(ver.to_string()));
    }

    #[test]
    fn save_global_creates_new_global_file_if_does_not_exist() {
        let global_file = assert_fs::NamedTempFile::new(FILE_NAME).unwrap();
        let current_dir = assert_fs::TempDir::new().unwrap();
        let (tool, version) = FIXTURE_TOOL1_GLOBAL;
        let tvs = ToolVersions::new(&global_file, &current_dir, tool);
        tvs.save_global(version).unwrap();
        let res = tvs.get_version().unwrap();
        assert_eq!(res, Some(version.to_string()), "saved and loaded version should match");
    }

    #[rstest]
    #[case(FIXTURE_GLOBAL, ("tool4", "1.0"), "test with new tool")]
    #[case(FIXTURE_GLOBAL, ("tool1", "1.4"), "test with updated tool")]
    fn save_global_saves_global_version(
        #[case] global: &str,
        #[case] tool_and_version: (&str, &str),
        #[case] msg: &str,
    ) {
        let global_file = assert_fs::NamedTempFile::new(FILE_NAME).unwrap();
        global_file.write_str(global).unwrap();
        let current_dir = assert_fs::TempDir::new().unwrap();
        let (tool, version) = tool_and_version;
        let tvs = ToolVersions::new(&global_file, &current_dir, tool);
        tvs.save_global(version).unwrap();
        let res = tvs.get_version().unwrap();

        assert_eq!(res, Some(version.to_string()), "{}: loaded does not match saved", msg);
    }

    #[test]
    fn save_local_creates_new_local_file_if_not_exists() {
        let global_file = assert_fs::NamedTempFile::new(FILE_NAME).unwrap();
        global_file.write_str(FIXTURE_GLOBAL).unwrap();
        let current_dir = assert_fs::TempDir::new().unwrap();
        let (tool, version) = FIXTURE_TOOL1_LOCAL;
        let tvs = ToolVersions::new(&global_file, &current_dir, tool);
        tvs.save_local(version).unwrap();
        let res = tvs.get_version().unwrap();
        assert_eq!(res, Some(version.to_string()), "saved and loaded version should match");
    }

    #[rstest]
    #[case(FIXTURE_TOOL2_LOCAL, "test set local tool")]
    #[case(("tool1", "v1.4"), "test update existing local tool")]
    fn save_local_sets_local_version_correctly(#[case] tool_and_version: (&str, &str), #[case] msg: &str) {
        let (global_file, current_dir) = gen_tool_versions_fixture();
        let (tool, version) = tool_and_version;
        let tvs = ToolVersions::new(&global_file, &current_dir, tool);
        tvs.save_local(version).unwrap();
        let res = tvs.get_version().unwrap();
        assert_eq!(res, Some(version.to_string()), "{}: loaded does not match saved", msg);
    }
}
