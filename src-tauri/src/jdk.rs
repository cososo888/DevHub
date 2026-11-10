//! Safe, user-owned JDK bundle management.
//! Never deletes files directly: a validated bundle is moved to the macOS Trash.

use serde::Serialize;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::process;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JdkInfo {
    pub root: String,
    pub vendor: String,
    pub can_uninstall: bool,
    pub reason: Option<String>,
    pub trash_path: Option<String>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_secs()).unwrap_or(0)
}

fn plist(root: &Path, key: &str) -> Option<String> {
    let info = root.join("Contents/Info.plist");
    if !info.is_file() { return None; }
    let arg = info.to_string_lossy().into_owned();
    let output = process::run(
        Path::new("/usr/libexec/PlistBuddy"),
        &["-c", &format!("Print :{key}"), &arg],
        Duration::from_secs(5),
        64 * 1024,
    ).ok()?;
    if output.status != 0 { return None; }
    let value = output.stdout.trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// Returns the bundle root for a JDK's `Contents/Home/bin/java` path.
fn bundle_root(java: &Path) -> Option<PathBuf> {
    let text = java.to_string_lossy();
    let root = text.strip_suffix("/Contents/Home/bin/java")?;
    Some(PathBuf::from(root))
}

fn allowed_location(root: &Path) -> bool {
    let text = root.to_string_lossy();
    let home = home();
    let home_text = home.to_string_lossy();
    if text.starts_with("/Library/Java/JavaVirtualMachines/") { return true; }
    if text.starts_with(&format!("{home_text}/Library/Java/JavaVirtualMachines/")) { return true; }
    // Oracle's self-extracted user bundle: ~/jdk-17.0.2.jdk
    if let Some(parent) = root.parent() {
        if parent == home && root.extension().and_then(|value| value.to_str()) == Some("jdk") { return true; }
    }
    false
}

pub fn analyze(java: &str) -> JdkInfo {
    let java_path = Path::new(java);
    let Some(root) = bundle_root(java_path) else {
        return JdkInfo { root: String::new(), vendor: "unknown".into(), can_uninstall: false, reason: Some("不是可识别的 JDK 安装路径".into()), trash_path: None };
    };
    let root_text = root.to_string_lossy().into_owned();
    let fail = |reason: &str| JdkInfo { root: root_text.clone(), vendor: "unknown".into(), can_uninstall: false, reason: Some(reason.into()), trash_path: None };
    if !allowed_location(&root) { return fail("不在允许的 JDK 安装目录内"); }
    let metadata = match fs::symlink_metadata(&root) { Ok(value) => value, Err(_) => return fail("无法读取 JDK 安装包") };
    if !metadata.is_dir() || metadata.file_type().is_symlink() { return fail("JDK 安装包不是完整目录"); }
    if metadata.uid() != unsafe { libc::getuid() } {
        return JdkInfo { root: root_text, vendor: "system".into(), can_uninstall: false, reason: Some("JDK 属于系统或其他用户，需要管理员权限".into()), trash_path: None };
    }
    if !root.join("Contents/Home/bin/java").is_file() || !root.join("Contents/Home/lib").is_dir() || !root.join("Contents/Info.plist").is_file() {
        return fail("JDK 安装包结构不完整");
    }
    let identifier = plist(&root, "CFBundleIdentifier").unwrap_or_default();
    let name = plist(&root, "CFBundleName").unwrap_or_default();
    let vendor = plist(&root, "JavaVM:JVMVendor").unwrap_or_else(|| name.clone());
    let id_lower = identifier.to_lowercase();
    let vendor_lower = vendor.to_lowercase();
    let is_jbr = id_lower.contains("jetbrains") || vendor_lower.contains("jetbrains") || name.to_lowercase().contains("jbr");
    if is_jbr {
        return JdkInfo { root: root_text, vendor: if vendor.is_empty() { "JetBrains".into() } else { vendor }, can_uninstall: false, reason: Some("JetBrains Runtime 可能被 IntelliJ IDEA 使用，暂不提供卸载".into()), trash_path: None };
    }
    let known = id_lower.contains("oracle") || id_lower.contains("openjdk") || id_lower.contains("microsoft") || vendor_lower.contains("oracle") || vendor_lower.contains("microsoft") || vendor_lower.contains("openjdk");
    if !known {
        return JdkInfo { root: root_text, vendor: if vendor.is_empty() { "unknown".into() } else { vendor }, can_uninstall: false, reason: Some("无法确认 JDK 发行版".into()), trash_path: None };
    }
    JdkInfo { root: root_text, vendor: if vendor.is_empty() { "OpenJDK".into() } else { vendor }, can_uninstall: true, reason: None, trash_path: None }
}

pub fn uninstall(java: &str) -> Result<JdkInfo, String> {
    let mut info = analyze(java);
    if !info.can_uninstall {
        return Err(info.reason.unwrap_or_else(|| "该 JDK 不能安全卸载".into()));
    }
    let root = PathBuf::from(&info.root);
    let trash = home().join(".Trash");
    fs::create_dir_all(&trash).map_err(|error| format!("无法访问废纸篓：{error}"))?;
    let base = root.file_name().and_then(|value| value.to_str()).unwrap_or("jdk");
    let mut target = trash.join(format!("DevHub-JDK-{}-{base}", now()));
    let mut suffix = 1;
    while target.exists() {
        target = trash.join(format!("DevHub-JDK-{}-{base}-{suffix}", now()));
        suffix += 1;
    }
    fs::rename(&root, &target).map_err(|error| format!("无法将 JDK 移入废纸篓：{error}"))?;
    info.trash_path = Some(target.to_string_lossy().into_owned());
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_root_is_derived_from_java_home_layout() {
        let root = bundle_root(Path::new("/Users/test/jdk-17.0.2.jdk/Contents/Home/bin/java")).expect("root");
        assert_eq!(root, Path::new("/Users/test/jdk-17.0.2.jdk"));
    }

    #[test]
    fn unknown_java_paths_are_not_uninstallable() {
        let info = analyze("/usr/bin/java");
        assert!(!info.can_uninstall);
        assert!(info.reason.is_some());
    }
}
