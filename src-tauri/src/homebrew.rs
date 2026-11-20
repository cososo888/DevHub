//! Thin, safe adapter over the installed Homebrew CLI.
//! DevHub never downloads or unpacks packages itself; every action is a fixed argv call.

use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::process;

const ENV: &[(&str, &str)] = &[
    ("HOMEBREW_NO_AUTO_UPDATE", "1"),
    ("HOMEBREW_NO_INSTALL_CLEANUP", "1"),
    ("HOMEBREW_NO_ENV_HINTS", "1"),
];
const LIST_TIMEOUT: Duration = Duration::from_secs(30);
const INFO_TIMEOUT: Duration = Duration::from_secs(45);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(1800);
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
// Curated GUI packages only. Resolve their declared app artifacts in one metadata query.
const CURATED_CASKS: &[&str] = &[
    "visual-studio-code", "docker", "intellij-idea", "rectangle", "raycast", "alfred",
    "cleanshot", "hiddenbar", "alt-tab", "stats", "monitorcontrol", "appcleaner",
    "blender", "gimp", "inkscape", "krita",
];

#[derive(Clone, Debug)]
pub struct Brew { pub path: PathBuf }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersion { pub package: String, pub version: String, pub path: String }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackages {
    pub formulae: Vec<PackageVersion>,
    pub casks: Vec<PackageVersion>,
    pub external_apps: Vec<ExternalApp>,
}

/// Presence at a declared application path is not Homebrew ownership or execution approval.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApp { pub package: String, pub path: String }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HomebrewResult {
    pub package: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub version: Option<String>,
}

pub fn probe() -> Result<Brew, String> {
    let path = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"].into_iter().map(PathBuf::from).find(|p| p.is_file())
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
        if args.first() == Some(&"install") && text.to_lowercase().contains("there is already an app at") {
            return Err(format!("已有应用，Homebrew 为避免覆盖而停止安装。请先查看已有应用。\n\n[Homebrew 原始输出]\n{text}"));
        }
        return Err(if text.is_empty() { "Homebrew 命令返回失败".into() } else { text });
    }
    Ok(format!("{}\n{}", output.stdout, output.stderr))
}

fn valid_kind(kind: &str) -> bool { kind == "formula" || kind == "cask" }
fn valid_package(package: &str) -> bool {
    let valid_part = |part: &str| {
        part.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
            && part.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'@' | b'+' | b'_' | b'-'))
    };
    package.len() <= 128 && matches!(package.split('/').count(), 1 | 3)
        && package.split('/').all(valid_part)
}

fn list_flag(kind: &str) -> &'static str { if kind == "cask" { "--cask" } else { "--formula" } }

pub fn installed_packages() -> Result<InstalledPackages, String> {
    let brew = probe()?;
    let prefix_output = process::run_with_env(&brew.path, &["--prefix"], Duration::from_secs(10), 64 * 1024, ENV)?;
    let prefix = String::from_utf8_lossy(prefix_output.stdout.as_bytes()).trim().to_string();
    let formula_output = process::run_with_env(&brew.path, &["list", "--formula", "--versions"], LIST_TIMEOUT, 2 * 1024 * 1024, ENV)?;
    if formula_output.status != 0 { return Err(format!("无法读取 Homebrew formula：{}", formula_output.stderr.trim())); }
    let cask_output = process::run_with_env(&brew.path, &["list", "--cask", "--versions"], LIST_TIMEOUT, 2 * 1024 * 1024, ENV)?;
    if cask_output.status != 0 { return Err(format!("无法读取 Homebrew cask：{}", cask_output.stderr.trim())); }
    let parse = |text: &str, kind: &str| -> Vec<PackageVersion> {
        text.lines().filter_map(|line| {
            let mut parts = line.split_whitespace();
            let package = parts.next()?.to_string();
            let version = parts.collect::<Vec<_>>().join(" ");
            let path = if kind == "cask" { format!("{prefix}/Caskroom/{package}") } else { format!("{prefix}/opt/{package}") };
            Some(PackageVersion { package, version: if version.is_empty() { "未知版本".into() } else { version }, path })
        }).collect()
    };
    let mut casks = parse(&cask_output.stdout, "cask");
    let external_apps = curated_external_apps(&brew, &mut casks)?;
    Ok(InstalledPackages {
        formulae: parse(&formula_output.stdout, "formula"),
        casks,
        external_apps,
    })
}

fn installed_versions_args(package: &str, kind: &str) -> Vec<String> {
    if kind == "cask" {
        vec!["list".into(), "--cask".into(), "--versions".into(), package.into()]
    } else {
        vec!["list".into(), "--formula".into(), "--versions".into(), package.into()]
    }
}

/// Parse a multi-formula JSON response and select the exact requested formula.
/// Never takes the first formula: `brew info --installed` can return several.
fn formula_installed_from_json(text: &str, package: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| format!("Homebrew JSON 无法解析：{error}"))?;
    let formulae = value.get("formulae").and_then(|value| value.as_array())
        .ok_or_else(|| "Homebrew 没有返回 formulae 数组".to_string())?;
    for formula in formulae {
        let name = formula.get("name").and_then(|value| value.as_str());
        let full_name = formula.get("full_name").and_then(|value| value.as_str());
        if name != Some(package) && full_name != Some(package) { continue; }
        return match formula.get("installed") {
            Some(serde_json::Value::Array(installed)) if installed.iter().all(serde_json::Value::is_object) => Ok(!installed.is_empty()),
            Some(_) => Err("Homebrew formula 的 installed 字段格式不正确".into()),
            None => match formula.get("linked_keg") {
                Some(serde_json::Value::String(version)) => Ok(!version.trim().is_empty()),
                _ => Err("Homebrew formula 缺少可确认的安装状态".into()),
            },
        };
    }
    Err(format!("Homebrew JSON 中没有精确匹配的 formula：{package}"))
}

/// Accept explicit cask version strings, null, or arrays of version strings.
/// Unknown or missing fields are query errors, never evidence of absence.
fn cask_installed_from_json(text: &str, package: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| format!("Homebrew JSON 无法解析：{error}"))?;
    let casks = value.get("casks").and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Homebrew 没有返回 casks 数组".to_string())?;
    for cask in casks {
        if !cask_matches(cask, package) { continue; }
        return cask_installed_state(cask);
    }
    Err(format!("Homebrew JSON 中没有精确匹配的 cask：{package}"))
}

fn cask_installed_state(cask: &serde_json::Value) -> Result<bool, String> {
    match cask.get("installed") {
        Some(serde_json::Value::Null) => Ok(false),
        Some(serde_json::Value::String(version)) if !version.trim().is_empty() => Ok(true),
        Some(serde_json::Value::Array(versions)) if versions.iter().all(|version| version.as_str().is_some_and(|s| !s.trim().is_empty())) => Ok(!versions.is_empty()),
        _ => Err("Homebrew cask 缺少可确认的安装状态".into()),
    }
}

fn cask_matches(cask: &serde_json::Value, package: &str) -> bool {
    cask.get("token").and_then(serde_json::Value::as_str) == Some(package)
        || cask.get("full_token").and_then(serde_json::Value::as_str) == Some(package)
        || cask.get("old_tokens").and_then(serde_json::Value::as_array)
            .is_some_and(|tokens| tokens.iter().any(|token| token.as_str() == Some(package)))
}

fn cask_info(brew: &Brew, packages: &[&str]) -> Result<serde_json::Value, String> {
    let mut args = vec!["info", "--json=v2", "--cask"];
    args.extend_from_slice(packages);
    let output = process::run_with_env(&brew.path, &args, INFO_TIMEOUT, OUTPUT_LIMIT, ENV)?;
    if output.status != 0 { return Err(query_failure("无法核对桌面应用目标", &output)); }
    serde_json::from_str(&output.stdout).map_err(|error| format!("桌面应用元数据无法解析：{error}"))
}

fn exact_cask<'a>(info: &'a serde_json::Value, package: &str) -> Result<&'a serde_json::Value, String> {
    let casks = info.get("casks").and_then(serde_json::Value::as_array)
        .ok_or("Homebrew 没有返回 casks 数组")?;
    let mut matching = casks.iter().filter(|cask| cask_matches(cask, package));
    let cask = matching.next().ok_or_else(|| format!("桌面应用元数据没有精确匹配：{package}"))?;
    if matching.next().is_some() { return Err(format!("桌面应用元数据匹配不唯一：{package}")); }
    Ok(cask)
}

fn app_filename(value: &str, source: bool) -> Result<String, String> {
    if value.is_empty() || value.chars().any(char::is_control) { return Err("应用目标名称无效".into()); }
    let path = Path::new(value);
    if path.components().any(|component| !matches!(component, Component::Normal(_))) {
        return Err("应用目标不能包含绝对路径或上级目录".into());
    }
    if !source && path.components().count() != 1 { return Err("应用目标必须是单个文件名".into()); }
    let name = path.file_name().and_then(|name| name.to_str()).ok_or("应用目标不是UTF-8")?;
    if !name.ends_with(".app") || name.len() <= 4 || name.len() > 255 { return Err("应用目标不是合法app名称".into()); }
    Ok(name.into())
}

fn declared_app_names(cask: &serde_json::Value) -> Result<Vec<String>, String> {
    let artifacts = cask.get("artifacts").and_then(serde_json::Value::as_array)
        .ok_or("桌面应用缺少 artifacts 元数据")?;
    let mut names = Vec::new();
    for artifact in artifacts {
        let Some(app) = artifact.get("app") else { continue };
        let app = app.as_array().ok_or("桌面应用的app目标格式不正确")?;
        let source = app.first().and_then(serde_json::Value::as_str).ok_or("桌面应用缺少app源名称")?;
        let mut name = app_filename(source, true)?;
        for option in app.iter().skip(1) {
            let option = option.as_object().ok_or("桌面应用目标选项格式不正确")?;
            if let Some(target) = option.get("target") {
                name = app_filename(target.as_str().ok_or("桌面应用目标名称格式不正确")?, false)?;
            }
        }
        if !names.contains(&name) { names.push(name); }
    }
    Ok(names)
}

/// Reject observed symlinks/non-directories component by component. Metadata-only presence:
/// this does not confer ownership or an atomic filesystem proof, and never executes the app.
fn ordinary_directory(path: &Path) -> Result<bool, String> {
    if !path.is_absolute() { return Err("应用检查根必须是绝对目录".into()); }
    let mut current = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => current.push(name),
            _ => return Err("应用检查路径不允许跳转目录".into()),
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() => (),
            Ok(_) => return Err(format!("已有应用或路径条目无法安全确认：{}；不会覆盖或跟随链接", current.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("无法读取应用路径元数据：{}：{error}", current.display())),
        }
    }
    Ok(true)
}

fn application_roots() -> Result<Vec<PathBuf>, String> {
    let mut roots = vec![PathBuf::from("/Applications")];
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("无法确认用户应用目录")?;
    if !home.is_absolute() { return Err("用户主目录不是绝对路径".into()); }
    roots.push(home.join("Applications"));
    Ok(roots)
}

fn existing_apps(cask: &serde_json::Value, package: &str) -> Result<Vec<ExternalApp>, String> {
    let names = declared_app_names(cask)?;
    let mut found = Vec::new();
    for root in application_roots()? {
        if !ordinary_directory(&root)? { continue; }
        for name in &names {
            let app = root.join(name);
            if !ordinary_directory(&app)? { continue; }
            if !ordinary_directory(&app.join("Contents"))? {
                return Err(format!("已有应用但结构不完整：{}；不会覆盖", app.display()));
            }
            let info = app.join("Contents/Info.plist");
            match std::fs::symlink_metadata(&info) {
                Ok(metadata) if metadata.file_type().is_file() => (),
                _ => return Err(format!("已有应用但Info.plist无法安全确认：{}；不会覆盖", app.display())),
            }
            found.push(ExternalApp { package: package.into(), path: app.to_string_lossy().into_owned() });
        }
    }
    Ok(found)
}

fn curated_external_apps(brew: &Brew, installed: &mut Vec<PackageVersion>) -> Result<Vec<ExternalApp>, String> {
    let info = cask_info(brew, CURATED_CASKS)?;
    let mut external = Vec::new();
    for package in CURATED_CASKS {
        let cask = exact_cask(&info, package)?;
        let token = cask.get("token").and_then(serde_json::Value::as_str).ok_or("桌面应用缺少token")?;
        if let Some(current) = installed.iter().find(|record| record.package == *package || record.package == token).cloned() {
            // Preserve official old-token aliases without treating a Homebrew app as external.
            if current.package != *package { installed.push(PackageVersion { package: (*package).into(), ..current }); }
            continue;
        }
        if cask_installed_state(cask)? {
            return Err(format!("桌面应用 {package} 的安装记录在查询期间变化，请刷新状态"));
        }
        external.extend(existing_apps(cask, package)?);
    }
    Ok(external)
}

/// Rechecked immediately before invoking Homebrew; no overwrite/adoption flags are used.
pub fn check_installation_target(package: &str, kind: &str) -> Result<(), String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    if kind != "cask" { return Ok(()); }
    let brew = probe()?;
    let info = cask_info(&brew, &[package])?;
    let cask = exact_cask(&info, package)?;
    if cask_installed_state(cask)? {
        return Err("Homebrew 已经安装该软件包，请刷新状态".into());
    }
    let existing = existing_apps(cask, package)?;
    if !existing.is_empty() {
        let paths = existing.iter().map(|app| app.path.as_str()).collect::<Vec<_>>().join("、");
        return Err(format!("已有应用：{paths}。该应用未登记为 Homebrew 管理，DevHub 不会覆盖。请先查看已有应用。"));
    }
    Ok(())
}

fn query_failure(context: &str, output: &process::Output) -> String {
    let details = format!("{}\n{}", output.stdout, output.stderr).trim().to_string();
    if details.is_empty() {
        format!("{context}（退出码 {}，Homebrew 未返回错误详情）", output.status)
    } else {
        format!("{context}（退出码 {}）\n{details}", output.status)
    }
}

fn fallback_package_json(brew: &Brew, package: &str, kind: &str) -> Result<bool, String> {
    let output = process::run_with_env(
        &brew.path,
        &["info", "--json=v2", list_flag(kind), package],
        INFO_TIMEOUT,
        OUTPUT_LIMIT,
        ENV,
    )?;
    if output.status != 0 {
        return Err(query_failure("Homebrew 安装状态 JSON 查询失败", &output));
    }
    if kind == "cask" {
        cask_installed_from_json(&output.stdout, package)
    } else {
        formula_installed_from_json(&output.stdout, package)
    }
}

/// Homebrew may return exit 1 and no text for an uninstalled package.
/// Both formula and cask need an exact JSON recheck; a nonzero code alone proves nothing.
pub fn package_installed(package: &str, kind: &str) -> Result<bool, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    let brew = probe()?;
    let args_owned = installed_versions_args(package, kind);
    let args = args_owned.iter().map(String::as_str).collect::<Vec<_>>();
    let primary = process::run_with_env(&brew.path, &args, LIST_TIMEOUT, 256 * 1024, ENV);
    let primary_message = match primary {
        Ok(output) => {
            let found = output.stdout.lines().any(|line| {
                let mut parts = line.split_whitespace();
                parts.next() == Some(package) && parts.next().is_some()
            });
            if output.status == 0 && found { return Ok(true); }
            if output.status == 0 {
                "Homebrew 版本列表没有精确匹配的安装记录".to_string()
            } else {
                query_failure("Homebrew 版本列表查询未确认安装状态", &output)
            }
        }
        Err(error) => format!("Homebrew 版本列表查询失败：{error}"),
    };
    fallback_package_json(&brew, package, kind)
        .map_err(|fallback| format!("{primary_message}\n\n[安装状态 JSON 复核]\n{fallback}"))
}

/// Verification after an installer can race Homebrew's state write.
/// Retry briefly, but never turn a query failure into "not installed".
pub fn package_installed_verified(package: &str, kind: &str) -> Result<bool, String> {
    let mut last_error = None;
    for attempt in 0..4 {
        match package_installed(package, kind) {
            Ok(true) => return Ok(true),
            Ok(false) => last_error = None,
            Err(error) => last_error = Some(error),
        }
        if attempt < 3 { std::thread::sleep(Duration::from_millis(250)); }
    }
    match last_error {
        Some(error) => Err(error),
        None => Ok(false),
    }
}

pub fn install_package(package: &str, kind: &str) -> Result<String, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    if package_installed(package, kind)? { return Err("Homebrew 已经安装该软件包".into()); }
    check_installation_target(package, kind)?;
    let brew = probe()?;
    let args = if kind == "cask" { vec!["install", "--cask", package] } else { vec!["install", "--formula", package] };
    run(&brew, &args, INSTALL_TIMEOUT)
}

pub fn uninstall_package(package: &str, kind: &str) -> Result<String, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    if !package_installed(package, kind)? { return Err("Homebrew 当前没有安装该软件包".into()); }
    let brew = probe()?;
    let args = if kind == "cask" { vec!["uninstall", "--cask", package] } else { vec!["uninstall", "--formula", package] };
    run(&brew, &args, UNINSTALL_TIMEOUT)
}

static SEARCH_CACHE: OnceLock<Mutex<std::collections::HashMap<String, (Instant, Vec<HomebrewResult>)>>> = OnceLock::new();

fn search_names(brew: &Brew, kind: &str, query: &str) -> Result<Vec<String>, String> {
    let args = if kind == "cask" { vec!["search", "--cask", query] } else { vec!["search", "--formula", query] };
    let output = process::run_with_env(&brew.path, &args, INFO_TIMEOUT, 2 * 1024 * 1024, ENV)?;
    if output.status != 0 {
        let expected = format!("Error: No formulae or casks found for \"{query}\".");
        if output.stdout.trim().is_empty() && output.stderr.trim() == expected {
            return Ok(Vec::new());
        }
        return Err(query_failure("Homebrew 搜索未完成", &output));
    }
    let mut names = output.stdout.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("==>"))
        .filter(|line| valid_package(line))
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Ok(names.into_iter().take(12).collect())
}

fn info_results(brew: &Brew, kind: &str, names: &[String]) -> Vec<HomebrewResult> {
    if names.is_empty() { return Vec::new(); }
    let mut args: Vec<&str> = vec!["info", "--json=v2", list_flag(kind)];
    args.extend(names.iter().map(String::as_str));
    let fallback = names.iter().map(|package| HomebrewResult {
        package: package.clone(), kind: kind.into(), name: package.clone(),
        description: "Homebrew 软件包".into(), version: None,
    }).collect::<Vec<_>>();
    let Ok(output) = process::run_with_env(&brew.path, &args, INFO_TIMEOUT, OUTPUT_LIMIT, ENV) else { return fallback };
    if output.status != 0 { return fallback; }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&output.stdout) else { return fallback };
    let key = if kind == "cask" { "casks" } else { "formulae" };
    let mut results = Vec::new();
    if let Some(values) = value.get(key).and_then(|v| v.as_array()) {
        for item in values {
            let package = if kind == "cask" { item.get("token").and_then(|v| v.as_str()) } else { item.get("name").and_then(|v| v.as_str()) };
            let Some(package) = package else { continue };
            let display = item.get("name").and_then(|v| {
                if let Some(s) = v.as_str() { Some(s.to_string()) } else { v.as_array().and_then(|a| a.first()).and_then(|x| x.as_str()).map(str::to_string) }
            }).unwrap_or_else(|| package.to_string());
            let description = item.get("desc").and_then(|v| v.as_str()).unwrap_or("Homebrew 软件包").to_string();
            let version = if kind == "cask" {
                item.get("version").and_then(|v| v.as_str()).map(str::to_string)
            } else {
                item.get("versions").and_then(|v| v.get("stable")).and_then(|v| v.as_str()).map(str::to_string)
            };
            results.push(HomebrewResult { package: package.to_string(), kind: kind.into(), name: display, description, version });
        }
    }
    if results.is_empty() { fallback } else { results }
}

pub fn search(query: &str) -> Result<Vec<HomebrewResult>, String> {
    let query = query.trim().to_lowercase();
    if query.len() < 2 || query.len() > 64 || query.starts_with(['-', '/']) || !query.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '@' | '/')) {
        return Err("请输入 2–64 个有效搜索字符".into());
    }
    let cache = SEARCH_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some((at, results)) = guard.get(&query) {
            if at.elapsed() < Duration::from_secs(300) { return Ok(results.clone()); }
        }
    }
    let brew = probe()?;
    let mut results = Vec::new();
    for kind in ["formula", "cask"] {
        let names = search_names(&brew, kind, &query)?;
        results.extend(info_results(&brew, kind, &names));
    }
    let mut seen = std::collections::HashSet::new();
    results.retain(|item| seen.insert(format!("{}:{}", item.kind, item.package)));
    results.truncate(24);
    if let Ok(mut guard) = cache.lock() { guard.insert(query, (Instant::now(), results.clone())); }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_validation_rejects_shell_metacharacters() {
        assert!(valid_package("node"));
        assert!(valid_package("python@3.13"));
        assert!(valid_package("visual-studio-code"));
        assert!(!valid_package("node; rm -rf ~"));
        assert!(!valid_package(""));
        assert!(valid_package("owner/tap/tool@2"));
        for invalid in ["--force", "/tmp/tool", "../tool", "owner/../tool", "owner/tap/-f"] {
            assert!(!valid_package(invalid));
        }
        assert!(!valid_kind("zip"));
    }

    #[test]
    fn installed_versions_query_uses_versions_not_unbounded_file_list() {
        let args = installed_versions_args("go", "formula");
        assert_eq!(args, vec!["list", "--formula", "--versions", "go"]);
        let cask = installed_versions_args("visual-studio-code", "cask");
        assert_eq!(cask, vec!["list", "--cask", "--versions", "visual-studio-code"]);
    }

    #[test]
    fn multi_formula_json_selects_exact_package_not_first() {
        let json = r#"{
          "formulae": [
            {"name":"fnm","full_name":"fnm","installed":[{"version":"1.39.0"}]},
            {"name":"go","full_name":"go","installed":[{"version":"1.27.3"}]},
            {"name":"ncmdump","full_name":"ncmdump","installed":[]}
          ]
        }"#;
        assert_eq!(formula_installed_from_json(json, "go").expect("go exact"), true);
        assert_eq!(formula_installed_from_json(json, "ncmdump").expect("ncmdump exact"), false);
        assert!(formula_installed_from_json(json, "does-not-exist").is_err());
    }

    #[test]
    #[ignore]
    fn live_installed_go_verification_is_true() {
        assert_eq!(package_installed("go", "formula").expect("go state"), true);
        assert_eq!(package_installed_verified("go", "formula").expect("go verified"), true);
    }

    #[test]
    #[ignore]
    fn live_search_returns_real_formula_and_cask_results() {
        let results = search("docker").expect("homebrew search");
        assert!(!results.is_empty());
        assert!(results.iter().any(|item| item.package == "docker"));
        assert!(results.iter().any(|item| item.kind == "formula" || item.kind == "cask"));
        for item in results {
            assert!(valid_package(&item.package));
            assert!(item.kind == "formula" || item.kind == "cask");
            assert!(!item.name.is_empty());
        }
        let installed = installed_packages().expect("installed packages");
        assert!(installed.formulae.iter().all(|item| !item.path.is_empty()));
    }

    #[test]
    fn installed_package_parser_handles_versions() {
        let parse = |text: &str| -> Vec<PackageVersion> {
            text.lines().filter_map(|line| {
                let mut parts = line.split_whitespace();
                let package = parts.next()?.to_string();
                let version = parts.collect::<Vec<_>>().join(" ");
                Some(PackageVersion { package, version, path: String::new() })
            }).collect()
        };
        let rows = parse("node 22.14.0\ngo 1.27.3\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].package, "node");
        assert_eq!(rows[1].version, "1.27.3");
    }
}
