//! Rust toolchain management through rustup. No project files or cargo caches are touched.

use std::path::Path;
use std::time::Duration;

use crate::process;

#[derive(Clone, Debug)]
pub struct RustupInfo {
    pub toolchain: String,
    pub can_uninstall: bool,
    pub reason: Option<String>,
}

fn rustup_bin() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home).join(".cargo/bin/rustup");
    if path.is_file() { Some(path) } else { None }
}

pub fn analyze(path: &str) -> RustupInfo {
    let home = std::env::var("HOME").unwrap_or_default();
    if !path.starts_with(&format!("{home}/.cargo/bin/")) {
        return RustupInfo { toolchain: String::new(), can_uninstall: false, reason: Some("不是 rustup 代理路径".into()) };
    }
    let Some(binary) = rustup_bin() else {
        return RustupInfo { toolchain: String::new(), can_uninstall: false, reason: Some("找不到 rustup 命令".into()) };
    };
    let output = match process::run(&binary, &["show", "active-toolchain"], Duration::from_secs(10), 64 * 1024) {
        Ok(value) if value.status == 0 => value,
        _ => return RustupInfo { toolchain: String::new(), can_uninstall: false, reason: Some("无法读取 rustup 活动工具链".into()) },
    };
    let toolchain = output.stdout.split_whitespace().next().unwrap_or("").to_string();
    if toolchain.is_empty() {
        return RustupInfo { toolchain, can_uninstall: false, reason: Some("没有可识别的 rustup 工具链".into()) };
    }
    RustupInfo { toolchain, can_uninstall: true, reason: None }
}

pub fn uninstall(path: &str) -> Result<String, String> {
    let info = analyze(path);
    if !info.can_uninstall {
        return Err(info.reason.unwrap_or_else(|| "该 rustup 工具链不能安全卸载".into()));
    }
    let Some(binary) = rustup_bin() else { return Err("找不到 rustup 命令".into()) };
    let output = process::run(&binary, &["toolchain", "uninstall", &info.toolchain], Duration::from_secs(120), 256 * 1024)
        .map_err(|error| format!("无法执行 rustup：{error}"))?;
    if output.status != 0 {
        return Err(format!("rustup 卸载失败：{}", output.stderr.trim()));
    }
    let _ = Path::new(path);
    Ok(info.toolchain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_rustup_paths_are_rejected() {
        let info = analyze("/usr/bin/rustc");
        assert!(!info.can_uninstall);
        assert!(info.reason.is_some());
    }
}
