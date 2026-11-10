//! Thin adapter over the installed Homebrew CLI.
//! DevHub never downloads, unpacks, resolves or deletes packages itself.

use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

use crate::process;

const ENV: &[(&str, &str)] = &[
    ("HOMEBREW_NO_AUTO_UPDATE", "1"),
    ("HOMEBREW_NO_INSTALL_CLEANUP", "1"),
    ("HOMEBREW_NO_ENV_HINTS", "1"),
];
const LIST_TIMEOUT: Duration = Duration::from_secs(20);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(1800);
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Brew {
    pub path: PathBuf,
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

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackagePlan {
    pub software_id: String,
    pub package: String,
    pub kind: String,
    pub install_command: String,
    pub uninstall_command: String,
    pub requirement: String,
}

pub fn plan(software_id: &str) -> Option<PackagePlan> {
    let (package, kind) = match software_id {
        "python" => ("python", "formula"),
        "node" => ("node", "formula"),
        "git" => ("git", "formula"),
        "vscode" => ("visual-studio-code", "cask"),
        "jdk" => ("openjdk", "formula"),
        "go" => ("go", "formula"),
        "docker" => ("docker", "cask"),
        "mysql" => ("mysql", "formula"),
        "postgresql" => ("postgresql@17", "formula"),
        "maven" => ("maven", "formula"),
        "gradle" => ("gradle", "formula"),
        "redis" => ("redis", "formula"),
        "rust" => ("rust", "formula"),
        "intellij" => ("intellij-idea", "cask"),
        _ => return None,
    };
    let install = if kind == "cask" { format!("brew install --cask {package}") } else { format!("brew install {package}") };
    let uninstall = if kind == "cask" { format!("brew uninstall --cask {package}") } else { format!("brew uninstall {package}") };
    Some(PackagePlan {
        software_id: software_id.into(),
        package: package.into(),
        kind: kind.into(),
        install_command: install,
        uninstall_command: uninstall,
        requirement: "Homebrew".into(),
    })
}

pub fn plans() -> Vec<PackagePlan> {
    ["python", "node", "git", "vscode", "jdk", "go", "docker", "mysql", "postgresql", "maven", "gradle", "redis", "rust", "intellij"]
        .into_iter().filter_map(plan).collect()
}

pub fn package_installed(plan: &PackagePlan) -> Result<bool, String> {
    let brew = probe()?;
    let args = if plan.kind == "cask" { vec!["list", "--cask", &plan.package] } else { vec!["list", "--formula", &plan.package] };
    let output = process::run_with_env(&brew.path, &args, LIST_TIMEOUT, 256 * 1024, ENV)?;
    Ok(output.status == 0)
}

pub fn install_package(plan: &PackagePlan) -> Result<String, String> {
    let brew = probe()?;
    let args = if plan.kind == "cask" { vec!["install", "--cask", &plan.package] } else { vec!["install", &plan.package] };
    run(&brew, &args, INSTALL_TIMEOUT)
}

pub fn uninstall_package(plan: &PackagePlan) -> Result<String, String> {
    let brew = probe()?;
    let args = if plan.kind == "cask" { vec!["uninstall", "--cask", &plan.package] } else { vec!["uninstall", &plan.package] };
    run(&brew, &args, UNINSTALL_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fourteen_software_have_fixed_package_plans() {
        let plans = plans();
        assert_eq!(plans.len(), 14);
        assert_eq!(plan("node").expect("node").install_command, "brew install node");
        assert_eq!(plan("vscode").expect("vscode").install_command, "brew install --cask visual-studio-code");
        assert_eq!(plan("postgresql").expect("postgresql").package, "postgresql@17");
        for plan in plans {
            assert!(!plan.install_command.contains("--zap"));
            assert!(!plan.install_command.contains("cleanup"));
            assert!(plan.install_command.starts_with("brew install "));
        }
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
if [ "$1" = "list" ]; then exit 0; fi
if [ "$1" = "install" ]; then echo "fake install stdout"; exit 0; fi
if [ "$1" = "uninstall" ]; then echo "fake uninstall stdout"; exit 0; fi
exit 0
"#).expect("script");
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("chmod");
        let brew = Brew { path: script };
        let output = run(&brew, &["install", "go"], Duration::from_secs(5)).expect("install");
        assert!(output.contains("fake install stdout"));
        assert!(fs::read_to_string(&log).expect("log").contains("install go"));
        let output = run(&brew, &["uninstall", "go"], Duration::from_secs(5)).expect("uninstall");
        assert!(output.contains("fake uninstall stdout"));
        let log_text = fs::read_to_string(&log).expect("log");
        assert!(log_text.contains("uninstall go"));
        assert!(!log_text.contains("--zap"));
        assert!(!log_text.contains("cleanup"));
        let _ = fs::remove_dir_all(&dir);
    }
}
