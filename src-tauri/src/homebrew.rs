//! Thin adapter over the installed Homebrew CLI.
//! DevHub never downloads, unpacks, resolves or deletes packages itself.

use serde::Serialize;
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
    pub prefix: String,
    pub version: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipView {
    pub id: i64,
    pub status: String,
    pub preexisting: bool,
    pub installed_at: i64,
    pub instance_path: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub software_id: String,
    pub package: Option<String>,
    pub package_type: Option<String>,
    pub brew_available: bool,
    pub brew_version: Option<String>,
    pub prefix: Option<String>,
    pub formula_version: Option<String>,
    pub channel: String,
    pub already_installed: bool,
    pub can_install: bool,
    pub reason: Option<String>,
    pub dependencies: Vec<String>,
    pub new_dependencies: Vec<String>,
    pub installed_dependencies: Vec<String>,
    pub ownership: Option<OwnershipView>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UninstallPreview {
    pub software_id: String,
    pub package: Option<String>,
    pub version: Option<String>,
    pub instance_path: Option<String>,
    pub can_uninstall: bool,
    pub reason: Option<String>,
    pub dependents: Vec<String>,
    pub ownership: Option<OwnershipView>,
    pub warnings: Vec<String>,
}

fn unsupported(software_id: &str, reason: &str) -> InstallPreview {
    InstallPreview {
        software_id: software_id.into(),
        package: None,
        package_type: None,
        brew_available: false,
        brew_version: None,
        prefix: None,
        formula_version: None,
        channel: "未接入".into(),
        already_installed: false,
        can_install: false,
        reason: Some(reason.into()),
        dependencies: Vec::new(),
        new_dependencies: Vec::new(),
        installed_dependencies: Vec::new(),
        ownership: None,
    }
}

pub fn package_for(software_id: &str) -> Option<(&'static str, &'static str)> {
    // Phase 3 intentionally starts with one reviewed formula. Do not turn this
    // into a generic package resolver.
    if software_id == "git" { Some(("git", "formula")) } else { None }
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
    Ok(Brew {
        path,
        prefix: prefix.stdout.trim().to_string(),
        version: version.stdout.lines().next().unwrap_or("").trim().trim_start_matches("Homebrew ").to_string(),
    })
}

fn run(brew: &Brew, args: &[&str], timeout: Duration) -> Result<String, String> {
    let output = process::run_with_env(&brew.path, args, timeout, OUTPUT_LIMIT, ENV)?;
    if output.status != 0 {
        let text = format!("{}\n{}", output.stdout, output.stderr).trim().to_string();
        return Err(if text.is_empty() { "Homebrew 命令返回失败".into() } else { text });
    }
    Ok(format!("{}\n{}", output.stdout, output.stderr))
}

fn info_for(brew: &Brew, package: &str) -> Result<Value, String> {
    let output = process::run_with_env(&brew.path, &["info", "--json=v2", package], INFO_TIMEOUT, OUTPUT_LIMIT, ENV)?;
    if output.status != 0 {
        return Err(format!("Homebrew 信息查询失败：{}", output.stderr.trim()));
    }
    serde_json::from_str(&output.stdout).map_err(|error| format!("Homebrew JSON 无法解析：{error}"))
}

fn installed_versions(brew: &Brew, package: &str) -> Result<Vec<String>, String> {
    // `brew list --versions` is read-only and exits successfully with no output when absent.
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

fn all_installed(brew: &Brew) -> Result<Vec<String>, String> {
    let output = process::run_with_env(&brew.path, &["list", "--versions"], LIST_TIMEOUT, 2 * 1024 * 1024, ENV)?;
    if output.status != 0 { return Ok(Vec::new()); }
    Ok(output.stdout.lines().filter_map(|line| line.split_whitespace().next().map(str::to_string)).collect())
}

fn dependents(brew: &Brew, package: &str) -> Result<Vec<String>, String> {
    let output = process::run_with_env(&brew.path, &["uses", "--installed", package], LIST_TIMEOUT, 512 * 1024, ENV)?;
    if output.status != 0 {
        return Err(format!("无法检查 Homebrew 依赖关系：{}", output.stderr.trim()));
    }
    Ok(output.stdout.lines().map(str::trim).filter(|line| !line.is_empty() && *line != package).map(str::to_string).collect())
}

pub fn preview_install(software_id: &str, ownership: Option<OwnershipView>) -> InstallPreview {
    let Some((package, package_type)) = package_for(software_id) else {
        return unsupported(software_id, "第三阶段先支持 Git 的 Homebrew 安装");
    };
    let brew = match probe() {
        Ok(value) => value,
        Err(error) => {
            let mut preview = unsupported(software_id, &error);
            preview.package = Some(package.into());
            preview.package_type = Some(package_type.into());
            return preview;
        }
    };
    let installed = installed_versions(&brew, package).unwrap_or_default();
    let already_installed = !installed.is_empty();
    let mut preview = InstallPreview {
        software_id: software_id.into(),
        package: Some(package.into()),
        package_type: Some(package_type.into()),
        brew_available: true,
        brew_version: Some(brew.version.clone()),
        prefix: Some(brew.prefix.clone()),
        formula_version: installed.first().cloned(),
        channel: "Homebrew 当前版本".into(),
        already_installed,
        can_install: false,
        reason: None,
        dependencies: Vec::new(),
        new_dependencies: Vec::new(),
        installed_dependencies: Vec::new(),
        ownership,
    };
    if already_installed {
        preview.reason = Some("Homebrew 已安装该包，DevHub 不会重复安装或自动接管".into());
        return preview;
    }
    let info = match info_for(&brew, package) {
        Ok(value) => value,
        Err(error) => {
            preview.reason = Some(format!("无法读取安装信息：{error}"));
            return preview;
        }
    };
    let formula = info.get("formulae").and_then(|value| value.as_array()).and_then(|values| values.first());
    preview.formula_version = formula
        .and_then(|value| value.get("versions"))
        .and_then(|value| value.get("stable"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    preview.dependencies = formula
        .and_then(|value| value.get("dependencies"))
        .and_then(|value| value.as_array())
        .map(|values| values.iter().filter_map(|value| value.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let installed_all = all_installed(&brew).unwrap_or_default();
    preview.installed_dependencies = preview.dependencies.iter().filter(|dep| installed_all.contains(dep)).cloned().collect();
    preview.new_dependencies = preview.dependencies.iter().filter(|dep| !installed_all.contains(dep)).cloned().collect();
    preview.can_install = true;
    preview.reason = None;
    preview
}

pub fn preview_uninstall(software_id: &str, ownership: Option<OwnershipView>) -> UninstallPreview {
    let Some((package, _)) = package_for(software_id) else {
        return UninstallPreview { software_id: software_id.into(), package: None, version: None, instance_path: None, can_uninstall: false, reason: Some("第三阶段先支持 Git 的 Homebrew 卸载".into()), dependents: Vec::new(), ownership, warnings: Vec::new() };
    };
    let brew = match probe() {
        Ok(value) => value,
        Err(error) => {
            return UninstallPreview { software_id: software_id.into(), package: Some(package.into()), version: None, instance_path: None, can_uninstall: false, reason: Some(error), dependents: Vec::new(), ownership, warnings: Vec::new() };
        }
    };
    let Some(ownership) = ownership else {
        return UninstallPreview { software_id: software_id.into(), package: Some(package.into()), version: None, instance_path: None, can_uninstall: false, reason: Some("该 Homebrew 包不是 DevHub 本次安装，DevHub 不会自动卸载外部包".into()), dependents: Vec::new(), ownership: None, warnings: Vec::new() };
    };
    if ownership.status != "success" || ownership.preexisting {
        return UninstallPreview { software_id: software_id.into(), package: Some(package.into()), version: ownership.version.clone(), instance_path: ownership.instance_path.clone(), can_uninstall: false, reason: Some("没有可安全卸载的 DevHub 安装归属记录".into()), dependents: Vec::new(), ownership: Some(ownership), warnings: Vec::new() };
    }
    let installed = installed_versions(&brew, package).unwrap_or_default();
    if installed.is_empty() {
        return UninstallPreview { software_id: software_id.into(), package: Some(package.into()), version: None, instance_path: ownership.instance_path.clone(), can_uninstall: false, reason: Some("Homebrew 当前没有安装该包".into()), dependents: Vec::new(), ownership: Some(ownership), warnings: Vec::new() };
    }
    let dependents = match dependents(&brew, package) {
        Ok(value) => value,
        Err(error) => {
            return UninstallPreview { software_id: software_id.into(), package: Some(package.into()), version: installed.first().cloned(), instance_path: ownership.instance_path.clone(), can_uninstall: false, reason: Some(error), dependents: Vec::new(), ownership: Some(ownership), warnings: Vec::new() };
        }
    };
    if !dependents.is_empty() {
        return UninstallPreview { software_id: software_id.into(), package: Some(package.into()), version: installed.first().cloned(), instance_path: ownership.instance_path.clone(), can_uninstall: false, reason: Some("该包被其他 Homebrew 包依赖，不能安全卸载".into()), dependents, ownership: Some(ownership), warnings: Vec::new() };
    }
    UninstallPreview {
        software_id: software_id.into(),
        package: Some(package.into()),
        version: installed.first().cloned(),
        instance_path: ownership.instance_path.clone(),
        can_uninstall: true,
        reason: None,
        dependents,
        ownership: Some(ownership),
        warnings: vec![
            "只卸载 Homebrew formula；不会执行 --zap、cleanup 或删除用户数据".into(),
            "其他来源的软件（例如系统 Git）不会被修改".into(),
        ],
    }
}

pub fn install(brew: &Brew, software_id: &str) -> Result<String, String> {
    let (package, _) = package_for(software_id).ok_or_else(|| "该软件没有可用的 Homebrew 安装计划".to_string())?;
    if !installed_versions(brew, package)?.is_empty() {
        return Err("Homebrew 已经安装该包，未重复执行安装".into());
    }
    run(brew, &["install", "--formula", package], INSTALL_TIMEOUT)
}

pub fn uninstall(brew: &Brew, software_id: &str) -> Result<String, String> {
    let (package, _) = package_for(software_id).ok_or_else(|| "该软件没有可用的 Homebrew 卸载计划".to_string())?;
    if installed_versions(brew, package)?.is_empty() {
        return Err("Homebrew 当前没有安装该包".into());
    }
    run(brew, &["uninstall", package], UNINSTALL_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_git_is_enabled_in_phase_three() {
        assert_eq!(package_for("git"), Some(("git", "formula")));
        assert_eq!(package_for("node"), None);
    }

    #[test]
    fn unsupported_preview_is_honest() {
        let preview = preview_install("node", None);
        assert!(!preview.can_install);
        assert!(preview.reason.is_some());
        assert!(!preview.already_installed);
    }

    #[test]
    fn install_and_uninstall_use_only_the_fixed_brew_commands() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("devhub-v2-brew-fixture-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture dir");
        let log = dir.join("log");
        let state = dir.join("state");
        let script = dir.join("brew");
        fs::write(&script, format!(r#"#!/bin/sh
DIR="$(dirname "$0")"
echo "$@" >> "$DIR/log"
if [ "$1" = "list" ] && [ "$2" = "--versions" ] && [ "$3" = "git" ]; then
  if [ -f "$DIR/state" ]; then cat "$DIR/state"; fi
  exit 0
fi
if [ "$1" = "install" ]; then
  echo "fake install stdout"
  echo "fake install stderr" >&2
  echo "git 2.51.0" > "$DIR/state"
  exit 0
fi
if [ "$1" = "uninstall" ]; then
  echo "fake uninstall stdout"
  rm -f "$DIR/state"
  exit 0
fi
exit 0
"#)).expect("script");
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("chmod");
        let brew = Brew { path: script.clone(), prefix: dir.to_string_lossy().into_owned(), version: "test".into() };
        let output = install(&brew, "git").expect("install command");
        assert!(output.contains("fake install stdout"));
        assert!(fs::read_to_string(&log).expect("log").contains("install --formula git"));
        let output = uninstall(&brew, "git").expect("uninstall command");
        assert!(output.contains("fake uninstall stdout"));
        let log_text = fs::read_to_string(&log).expect("log");
        assert!(log_text.contains("uninstall git"));
        assert!(!log_text.contains("--zap"));
        assert!(!log_text.contains("cleanup"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore]
    fn live_git_preview_report() {
        let preview = preview_install("git", None);
        println!("{}", serde_json::to_string_pretty(&preview).expect("preview json"));
    }

    #[test]
    fn uninstall_requires_ownership() {
        let preview = preview_uninstall("git", None);
        assert!(!preview.can_uninstall);
        assert!(preview.reason.unwrap().contains("不是 DevHub"));
    }
}
