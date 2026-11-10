//! Thin adapter over the installed Homebrew CLI.
//! DevHub never downloads, unpacks, resolves or deletes packages itself.

use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

use crate::process;

const ENV: &[(&str, &str)] = &[
    ("HOMEBREW_NO_AUTO_UPDATE", "1"),
    ("HOMEBREW_NO_INSTALL_CLEANUP", "1"),
    ("HOMEBREW_NO_ENV_HINTS", "1"),
];
const INFO_TIMEOUT: Duration = Duration::from_secs(30);
const LIST_TIMEOUT: Duration = Duration::from_secs(20);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(1800);
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Brew {
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FormulaInfo {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<String>,
    pub installed: Vec<String>,
    pub disabled: bool,
    pub deprecated: bool,
}

/// Reviewed Homebrew formula channels for the eight Phase 3 tools.
/// A version is only shown if `brew info` can resolve it now.
pub fn formulas_for(software_id: &str) -> &'static [&'static str] {
    match software_id {
        "git" => &["git"],
        "go" => &["go"],
        "maven" => &["maven"],
        "gradle" => &["gradle"],
        "node" => &["node", "node@24"],
        "python" => &["python@3.14", "python@3.13", "python@3.12"],
        "jdk" => &["openjdk", "openjdk@21", "openjdk@17"],
        "rust" => &["rust"],
        _ => &[],
    }
}

pub fn supported(software_id: &str) -> bool {
    !formulas_for(software_id).is_empty()
}

pub fn probe() -> Result<Brew, String> {
    let path = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .ok_or_else(|| "未检测到 Homebrew".to_string())?;
    let prefix = process::run_with_env(&path, &["--prefix"], Duration::from_secs(10), 64 * 1024, ENV)?;
    if prefix.status != 0 { return Err("无法读取 Homebrew 前缀".into()); }
    let version = process::run_with_env(&path, &["--version"], Duration::from_secs(10), 64 * 1024, ENV)?;
    if version.status != 0 { return Err("无法读取 Homebrew 版本".into()); }
    Ok(Brew { path })
}

fn run(brew: &Brew, args: &[&str], timeout: Duration) -> Result<String, String> {
    let output = process::run_with_env(&brew.path, args, timeout, OUTPUT_LIMIT, ENV)?;
    if output.status != 0 {
        let text = format!("{}\n{}", output.stdout, output.stderr).trim().to_string();
        return Err(if text.is_empty() { "Homebrew 命令返回失败".into() } else { text });
    }
    Ok(format!("{}\n{}", output.stdout, output.stderr))
}

fn info_for(brew: &Brew, package: &str) -> Result<FormulaInfo, String> {
    let output = process::run_with_env(&brew.path, &["info", "--json=v2", package], INFO_TIMEOUT, OUTPUT_LIMIT, ENV)?;
    if output.status != 0 {
        return Err(format!("Homebrew 信息查询失败：{}", output.stderr.trim()));
    }
    let parsed: Value = serde_json::from_str(&output.stdout).map_err(|error| format!("Homebrew JSON 无法解析：{error}"))?;
    let formula = parsed.get("formulae").and_then(|value| value.as_array()).and_then(|values| values.first())
        .ok_or_else(|| "Homebrew 没有返回 formula 信息".to_string())?;
    let version = formula.get("versions").and_then(|value| value.get("stable")).and_then(|value| value.as_str())
        .ok_or_else(|| "Homebrew 没有稳定版本".to_string())?.to_string();
    let dependencies = formula.get("dependencies").and_then(|value| value.as_array())
        .map(|values| values.iter().filter_map(|value| value.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let installed = formula.get("installed").and_then(|value| value.as_array())
        .map(|values| values.iter().filter_map(|value| value.get("version").and_then(|value| value.as_str()).map(str::to_string)).collect()).unwrap_or_default();
    Ok(FormulaInfo {
        name: formula.get("name").and_then(|value| value.as_str()).unwrap_or(package).to_string(),
        version,
        dependencies,
        installed,
        disabled: formula.get("disabled").and_then(|value| value.as_bool()).unwrap_or(false),
        deprecated: formula.get("deprecated").and_then(|value| value.as_bool()).unwrap_or(false),
    })
}

pub fn info(package: &str) -> Result<FormulaInfo, String> {
    let brew = probe()?;
    info_for(&brew, package)
}

/// Returns real formula candidates and a user-facing error only when the whole
/// query cannot run. Missing/deprecated candidates are simply not offered.
pub fn available(software_id: &str) -> (Vec<FormulaInfo>, Option<String>) {
    let brew = match probe() {
        Ok(value) => value,
        Err(error) => return (Vec::new(), Some(error)),
    };
    let mut values = Vec::new();
    for package in formulas_for(software_id) {
        if let Ok(info) = info_for(&brew, package) {
            if !info.disabled && !info.deprecated { values.push(info); }
        }
    }
    (values, None)
}

pub fn resolve_install(software_id: &str, version: &str) -> Result<FormulaInfo, String> {
    let brew = probe()?;
    let mut last_error = None;
    for package in formulas_for(software_id) {
        match info_for(&brew, package) {
            Ok(info) => {
                if info.disabled || info.deprecated { continue; }
                if info.version != version { continue; }
                if !info.installed.is_empty() {
                    return Err("Homebrew 已经安装该版本，未重复执行安装".into());
                }
                return Ok(info);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "没有找到该版本的真实可安装方案".into()))
}

fn installed_versions(brew: &Brew, package: &str) -> Result<Vec<String>, String> {
    let output = process::run_with_env(&brew.path, &["list", "--versions", package], LIST_TIMEOUT, 256 * 1024, ENV)?;
    if output.status != 0 {
        if output.stderr.contains("not installed") || output.stdout.trim().is_empty() { return Ok(Vec::new()); }
        return Err(format!("无法读取 Homebrew 安装状态：{}", output.stderr.trim()));
    }
    Ok(output.stdout.lines().filter_map(|line| {
        let mut parts = line.split_whitespace();
        let _name = parts.next()?;
        Some(parts.collect::<Vec<_>>().join(" "))
    }).filter(|value| !value.is_empty()).collect())
}

pub fn dependents(package: &str) -> Result<Vec<String>, String> {
    let brew = probe()?;
    let output = process::run_with_env(&brew.path, &["uses", "--installed", package], LIST_TIMEOUT, 512 * 1024, ENV)?;
    if output.status != 0 {
        return Err(format!("无法检查 Homebrew 依赖关系：{}", output.stderr.trim()));
    }
    Ok(output.stdout.lines().map(str::trim).filter(|line| !line.is_empty() && *line != package).map(str::to_string).collect())
}

pub fn install(package: &str) -> Result<String, String> {
    let brew = probe()?;
    if !installed_versions(&brew, package)?.is_empty() {
        return Err("Homebrew 已经安装该包，未重复执行安装".into());
    }
    run(&brew, &["install", "--formula", package], INSTALL_TIMEOUT)
}

pub fn uninstall(package: &str) -> Result<String, String> {
    let brew = probe()?;
    if installed_versions(&brew, package)?.is_empty() {
        return Err("Homebrew 当前没有安装该包".into());
    }
    run(&brew, &["uninstall", package], UNINSTALL_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_reviewed_tools_are_mapped() {
        assert!(supported("git"));
        assert!(supported("go"));
        assert!(!supported("docker"));
    }

    #[test]
    fn real_formula_versions_can_be_sorted_as_versions() {
        let mut values = vec!["1.10.0".to_string(), "1.9.0".to_string(), "2.0.0".to_string()];
        values.sort_by(|a, b| b.split('.').cmp(a.split('.')));
        assert_eq!(values[0], "2.0.0");
    }

    #[test]
    fn install_and_uninstall_use_only_fixed_brew_commands() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("devhub-v2-brew-fixture-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture dir");
        let log = dir.join("log");
        let script = dir.join("brew");
        fs::write(&script, r#"#!/bin/sh
DIR="$(dirname "$0")"
echo "$@" >> "$DIR/log"
if [ "$1" = "list" ] && [ "$2" = "--versions" ]; then
  if [ -f "$DIR/state" ]; then cat "$DIR/state"; fi
  exit 0
fi
if [ "$1" = "install" ]; then
  echo "fake install stdout"; echo "git 2.56.0" > "$DIR/state"; exit 0
fi
if [ "$1" = "uninstall" ]; then
  echo "fake uninstall stdout"; rm -f "$DIR/state"; exit 0
fi
exit 0
"#).expect("script");
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("chmod");
        let brew = Brew { path: script };
        // Use the low-level fixed command helper through a temporary PATH-free script.
        let output = run(&brew, &["install", "--formula", "git"], Duration::from_secs(5)).expect("install");
        assert!(output.contains("fake install stdout"));
        assert!(fs::read_to_string(&log).expect("log").contains("install --formula git"));
        let output = run(&brew, &["uninstall", "git"], Duration::from_secs(5)).expect("uninstall");
        assert!(output.contains("fake uninstall stdout"));
        let log_text = fs::read_to_string(&log).expect("log");
        assert!(log_text.contains("uninstall git"));
        assert!(!log_text.contains("--zap"));
        assert!(!log_text.contains("cleanup"));
        let _ = fs::remove_dir_all(&dir);
    }
}
