//! Node.js version management through fnm. Does not modify the user's shell or PATH.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::process;

fn binary() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("/opt/homebrew/bin/fnm"),
        PathBuf::from("/usr/local/bin/fnm"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/bin/fnm"),
    ];
    candidates.into_iter().find(|path| path.is_file())
}

fn parse_version(line: &str) -> Option<String> {
    let value = line.trim().trim_start_matches('*').trim().split_whitespace().next()?.trim_start_matches('v');
    if value.is_empty() || value.contains('-') { return None; }
    if !value.chars().all(|character| character.is_ascii_digit() || character == '.') { return None; }
    let major = value.split('.').next()?.parse::<u32>().ok()?;
    // fnm's remote index includes ancient versions without macOS arm64 builds.
    if major < 16 { return None; }
    Some(value.to_string())
}

static CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>)>>> = OnceLock::new();

pub fn available() -> Result<Vec<String>, String> {
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some((at, versions)) = guard.as_ref() {
            if at.elapsed() < Duration::from_secs(600) { return Ok(versions.clone()); }
        }
    }
    let Some(binary) = binary() else { return Err("未安装 fnm".into()) };
    let output = process::run(&binary, &["ls-remote"], Duration::from_secs(45), 2 * 1024 * 1024)
        .map_err(|error| format!("无法读取 Node 版本目录：{error}"))?;
    if output.status != 0 { return Err(format!("fnm ls-remote 失败：{}", output.stderr.trim())); }
    let mut versions = output.stdout.lines().filter_map(parse_version).collect::<Vec<_>>();
    versions.sort_by(|a, b| crate::versions::compare_version_strings(b, a));
    versions.dedup();
    if let Ok(mut guard) = cache.lock() { *guard = Some((Instant::now(), versions.clone())); }
    Ok(versions)
}

/// Latest patch per major is the UI default: keeps the minimal list short while
/// preserving real multi-major choice. The full fnm list remains used for validation.
pub fn catalog_versions() -> Result<Vec<String>, String> {
    let all = available()?;
    let mut majors = std::collections::HashSet::new();
    Ok(all.into_iter().filter(|version| {
        version.split('.').next().and_then(|major| major.parse::<u32>().ok()).map(|major| majors.insert(major)).unwrap_or(false)
    }).collect())
}

pub fn default_version() -> Option<String> {
    let binary = binary()?;
    let output = process::run(&binary, &["list"], Duration::from_secs(10), 64 * 1024).ok()?;
    if output.status != 0 { return None; }
    output.stdout.lines().find(|line| line.contains("default")).and_then(|line| {
        line.split_whitespace().find_map(|token| token.trim_start_matches('v').split('.').next().and_then(|major| major.parse::<u32>().ok()).map(|_| token.trim_start_matches('v').to_string()))
    })
}

pub fn validate_uninstall(version: &str, instance_path: &str) -> Result<(), String> {
    if !is_managed(instance_path) {
        return Err("目标不是 fnm 管理的安装".into());
    }
    let Some(expected) = version_root(version) else {
        return Err(format!("fnm 未安装 Node.js {version}"));
    };
    if expected.to_string_lossy() != instance_path {
        return Err("版本号与安装实例路径不匹配".into());
    }
    if default_version().as_deref() == Some(version) {
        return Err("该版本是 fnm 默认别名，卸载会修改默认别名；已阻止".into());
    }
    Ok(())
}

pub fn version_root(version: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    for base in [home.join("Library/Application Support/fnm/node-versions"), home.join(".local/share/fnm/node-versions")] {
        let path = base.join(format!("v{version}")).join("installation/bin/node");
        if path.is_file() { return Some(path); }
    }
    None
}

pub fn is_managed(path: &str) -> bool {
    path.contains("/fnm/node-versions/")
}

pub fn install(version: &str) -> Result<String, String> {
    let Some(binary) = binary() else { return Err("未安装 fnm".into()) };
    let output = process::run(&binary, &["install", version], Duration::from_secs(1800), 2 * 1024 * 1024)
        .map_err(|error| format!("无法执行 fnm：{error}"))?;
    if output.status != 0 { return Err(format!("fnm 安装失败：{}", output.stderr.trim())); }
    Ok(output.stdout)
}

pub fn uninstall(version: &str) -> Result<String, String> {
    let Some(binary) = binary() else { return Err("未安装 fnm".into()) };
    let output = process::run(&binary, &["uninstall", version], Duration::from_secs(60), 256 * 1024)
        .map_err(|error| format!("无法执行 fnm：{error}"))?;
    if output.status != 0 { return Err(format!("fnm 卸载失败：{}", output.stderr.trim())); }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn validates_real_non_default_install_without_uninstalling() {
        let default = default_version().expect("fnm default");
        let default_path = version_root(&default).expect("default path");
        assert!(validate_uninstall(&default, &default_path.to_string_lossy()).is_err());
        let version = "22.23.4";
        assert!(validate_uninstall(version, &default_path.to_string_lossy()).is_err());
        if let Some(path) = version_root(version) {
            assert!(validate_uninstall(version, &path.to_string_lossy()).is_ok());
            assert!(validate_uninstall(&default, &path.to_string_lossy()).is_err());
        }
    }

    #[test]
    fn parses_stable_fnm_versions() {
        assert_eq!(parse_version("v22.14.0"), Some("22.14.0".into()));
        assert_eq!(parse_version("* v24.1.0 default"), Some("24.1.0".into()));
        assert_eq!(parse_version("v27.0.0-alpha.0.0.0"), None);
        assert_eq!(parse_version("v14.21.3"), None);
    }
}
