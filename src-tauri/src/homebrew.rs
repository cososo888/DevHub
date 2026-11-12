//! Thin, safe adapter over the installed Homebrew CLI.
//! DevHub never downloads or unpacks packages itself; every action is a fixed argv call.

use serde::Serialize;
use std::path::PathBuf;
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
}

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
        return Err(if text.is_empty() { "Homebrew 命令返回失败".into() } else { text });
    }
    Ok(format!("{}\n{}", output.stdout, output.stderr))
}

fn valid_kind(kind: &str) -> bool { kind == "formula" || kind == "cask" }
fn valid_package(package: &str) -> bool {
    !package.is_empty() && package.len() <= 128
        && package.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '@' | '+' | '_' | '-' | '/'))
}

fn list_flag(kind: &str) -> &'static str { if kind == "cask" { "--cask" } else { "--formula" } }

pub fn installed_packages() -> Result<InstalledPackages, String> {
    let brew = probe()?;
    let prefix_output = process::run_with_env(&brew.path, &["--prefix"], Duration::from_secs(10), 64 * 1024, ENV)?;
    let prefix = String::from_utf8_lossy(prefix_output.stdout.as_bytes()).trim().to_string();
    let formula_output = process::run_with_env(&brew.path, &["list", "--formula", "--versions"], LIST_TIMEOUT, 2 * 1024 * 1024, ENV)?;
    if formula_output.status != 0 { return Err(format!("无法读取 Homebrew formula：{}", formula_output.stderr.trim())); }
    let cask_output = process::run_with_env(&brew.path, &["list", "--cask", "--versions"], LIST_TIMEOUT, 2 * 1024 * 1024, ENV)?;
    let parse = |text: &str, kind: &str| -> Vec<PackageVersion> {
        text.lines().filter_map(|line| {
            let mut parts = line.split_whitespace();
            let package = parts.next()?.to_string();
            let version = parts.collect::<Vec<_>>().join(" ");
            let path = if kind == "cask" { format!("{prefix}/Caskroom/{package}") } else { format!("{prefix}/opt/{package}") };
            Some(PackageVersion { package, version: if version.is_empty() { "未知版本".into() } else { version }, path })
        }).collect()
    };
    Ok(InstalledPackages {
        formulae: parse(&formula_output.stdout, "formula"),
        casks: if cask_output.status == 0 { parse(&cask_output.stdout, "cask") } else { Vec::new() },
    })
}

fn installed_versions_args(package: &str, kind: &str) -> Vec<String> {
    if kind == "cask" {
        vec!["list".into(), "--cask".into(), "--versions".into(), package.into()]
    } else {
        vec!["list".into(), "--formula".into(), "--versions".into(), package.into()]
    }
}

fn is_not_installed_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("not installed") || lower.contains("no such keg") || lower.contains("no installed keg")
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
        let installed = formula.get("installed").and_then(|value| value.as_array())
            .map(|values| !values.is_empty())
            .or_else(|| formula.get("linked_keg").and_then(|value| value.as_str()).map(|value| !value.is_empty()))
            .unwrap_or(false);
        return Ok(installed);
    }
    Err(format!("Homebrew JSON 中没有精确匹配的 formula：{package}"))
}

fn fallback_formula_json(brew: &Brew, package: &str) -> Result<bool, String> {
    let output = process::run_with_env(
        &brew.path,
        &["info", "--json=v2", "--formula", package],
        INFO_TIMEOUT,
        OUTPUT_LIMIT,
        ENV,
    )?;
    if output.status != 0 {
        return Err(format!("Homebrew 状态查询失败：{}", output.stderr.trim()));
    }
    formula_installed_from_json(&output.stdout, package)
}

/// Checks the exact installed package, not merely a process exit code.
/// The fallback is deliberately JSON-based and matches the exact formula name.
pub fn package_installed(package: &str, kind: &str) -> Result<bool, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    let brew = probe()?;
    let args_owned = installed_versions_args(package, kind);
    let args = args_owned.iter().map(String::as_str).collect::<Vec<_>>();
    match process::run_with_env(&brew.path, &args, LIST_TIMEOUT, 256 * 1024, ENV) {
        Ok(output) if output.status == 0 => {
            let found = output.stdout.lines().any(|line| line.split_whitespace().next() == Some(package));
            if found { return Ok(true); }
            // A zero exit with no exact matching package: confirm via JSON for formula.
            if kind == "formula" { return fallback_formula_json(&brew, package); }
            Ok(false)
        }
        Ok(output) if is_not_installed_text(&format!("{}\n{}", output.stdout, output.stderr)) => Ok(false),
        Ok(_) => {
            if kind == "formula" { fallback_formula_json(&brew, package) }
            else { Err("Homebrew 状态查询返回非零状态".into()) }
        }
        Err(primary_error) => {
            if kind == "formula" {
                fallback_formula_json(&brew, package).map_err(|fallback| format!("{primary_error}; {fallback}"))
            } else {
                Err(primary_error)
            }
        }
    }
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

pub fn package_exists(package: &str, kind: &str) -> Result<bool, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    let brew = probe()?;
    let output = process::run_with_env(&brew.path, &["info", "--json=v2", list_flag(kind), package], INFO_TIMEOUT, OUTPUT_LIMIT, ENV)?;
    Ok(output.status == 0 && output.stdout.trim().starts_with('{'))
}

pub fn install_package(package: &str, kind: &str) -> Result<String, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    if !package_exists(package, kind)? { return Err("Homebrew 中找不到该软件包".into()); }
    if package_installed(package, kind)? { return Err("Homebrew 已经安装该软件包".into()); }
    let brew = probe()?;
    let args = if kind == "cask" { vec!["install", "--cask", package] } else { vec!["install", package] };
    run(&brew, &args, INSTALL_TIMEOUT)
}

pub fn uninstall_package(package: &str, kind: &str) -> Result<String, String> {
    if !valid_kind(kind) || !valid_package(package) { return Err("无效的 Homebrew 包标识".into()); }
    if !package_installed(package, kind)? { return Err("Homebrew 当前没有安装该软件包".into()); }
    let brew = probe()?;
    let args = if kind == "cask" { vec!["uninstall", "--cask", package] } else { vec!["uninstall", package] };
    run(&brew, &args, UNINSTALL_TIMEOUT)
}

static SEARCH_CACHE: OnceLock<Mutex<std::collections::HashMap<String, (Instant, Vec<HomebrewResult>)>>> = OnceLock::new();

fn search_names(brew: &Brew, kind: &str, query: &str) -> Vec<String> {
    let args = if kind == "cask" { vec!["search", "--cask", query] } else { vec!["search", "--formula", query] };
    let Ok(output) = process::run_with_env(&brew.path, &args, INFO_TIMEOUT, 2 * 1024 * 1024, ENV) else { return Vec::new() };
    if output.status != 0 && output.stdout.trim().is_empty() { return Vec::new(); }
    let mut names = output.stdout.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("==>"))
        .filter(|line| valid_package(line))
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names.into_iter().take(12).collect()
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
    if query.len() < 2 || query.len() > 64 || !query.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '@' | '/')) {
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
        let names = search_names(&brew, kind, &query);
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
