//! Python versions managed by uv. No shell integration or PATH changes.

use std::path::PathBuf;
use std::time::Duration;

use crate::process;

fn binary() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    [home.join(".local/bin/uv"), PathBuf::from("/opt/homebrew/bin/uv"), PathBuf::from("/usr/local/bin/uv")].into_iter().find(|path| path.is_file())
}

fn parse_version(line: &str) -> Option<String> {
    let token = line.split_whitespace().next()?;
    let rest = token.strip_prefix("cpython-")?;
    let version = rest.split('-').next()?.to_string();
    if version.contains('+') || version.contains('a') || version.contains('b') || version.contains("rc") { return None; }
    if version.split('.').count() != 3 { return None; }
    Some(version)
}

pub fn available() -> Result<Vec<String>, String> {
    let Some(binary) = binary() else { return Err("未安装 uv".into()) };
    let output = process::run(&binary, &["python", "list", "--all-versions"], Duration::from_secs(45), 4 * 1024 * 1024)
        .map_err(|error| format!("无法读取 uv Python 目录：{error}"))?;
    if output.status != 0 { return Err(format!("uv python list 失败：{}", output.stderr.trim())); }
    let mut versions = output.stdout.lines().filter_map(parse_version).collect::<Vec<_>>();
    versions.sort_by(|a, b| crate::versions::compare_version_strings(b, a));
    versions.dedup();
    Ok(versions)
}

pub fn is_managed(path: &str) -> bool {
    path.contains("/.local/share/uv/python/")
}

pub fn version_root(version: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let base = home.join(".local/share/uv/python");
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let version_part = name.strip_prefix("cpython-")?.split('-').next()?;
        if version == version_part || version.starts_with(&format!("{version_part}.")) {
            let candidate = entry.path().join("bin/python3");
            if candidate.is_file() { return Some(candidate); }
        }
    }
    None
}

pub fn install(version: &str) -> Result<String, String> {
    let Some(binary) = binary() else { return Err("未安装 uv".into()) };
    let output = process::run(&binary, &["python", "install", version], Duration::from_secs(300), 2 * 1024 * 1024)
        .map_err(|error| format!("无法执行 uv：{error}"))?;
    if output.status != 0 { return Err(format!("uv Python 安装失败：{}", output.stderr.trim())); }
    Ok(output.stdout)
}

pub fn uninstall(version: &str) -> Result<String, String> {
    let Some(binary) = binary() else { return Err("未安装 uv".into()) };
    let output = process::run(&binary, &["python", "uninstall", version], Duration::from_secs(60), 256 * 1024)
        .map_err(|error| format!("无法执行 uv：{error}"))?;
    if output.status != 0 { return Err(format!("uv Python 卸载失败：{}", output.stderr.trim())); }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stable_uv_versions() {
        assert_eq!(parse_version("cpython-3.13.18-macos-aarch64-none <download available>"), Some("3.13.18".into()));
        assert_eq!(parse_version("cpython-3.14.8+freethreaded-macos-aarch64-none"), None);
    }
}
