//! Read-only macOS development-tool detection.
//! No install, upgrade, uninstall, cleanup, PATH or shell mutation is performed here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::process as safe_process;

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const PROBE_LIMIT: usize = 512 * 1024;
const BREW_TIMEOUT: Duration = Duration::from_secs(30);
const BREW_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareInstance {
    pub software_id: String,
    pub version: Option<String>,
    pub source: String,
    pub sources: Vec<String>,
    pub path: String,
    pub kind: String,
    pub executable: bool,
    pub managed_by: String,
    pub health: String,
    pub detail: Option<String>,
    pub last_detected_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HomebrewStatus {
    pub available: bool,
    pub prefix: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectionError {
    pub source: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectionSnapshot {
    pub schema_version: u32,
    pub refreshed_at: Option<i64>,
    pub stale: bool,
    pub duration_ms: u64,
    pub homebrew: HomebrewStatus,
    pub instances: Vec<SoftwareInstance>,
    pub errors: Vec<DetectionError>,
}

impl DetectionSnapshot {
    pub fn failed(message: String) -> Self {
        Self {
            schema_version: 1,
            refreshed_at: Some(now_ms()),
            stale: true,
            duration_ms: 0,
            homebrew: HomebrewStatus { available: false, prefix: None, version: None, error: None },
            instances: Vec::new(),
            errors: vec![DetectionError { source: "detector".into(), message }],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Tool {
    id: &'static str,
    commands: &'static [&'static str],
    arguments: &'static [&'static str],
}

const TOOLS: &[Tool] = &[
    Tool { id: "python", commands: &["python3", "python"], arguments: &["--version"] },
    Tool { id: "node", commands: &["node"], arguments: &["--version"] },
    Tool { id: "git", commands: &["git"], arguments: &["--version"] },
    Tool { id: "jdk", commands: &["java"], arguments: &["-version"] },
    Tool { id: "go", commands: &["go"], arguments: &["version"] },
    Tool { id: "maven", commands: &["mvn"], arguments: &["-version"] },
    Tool { id: "gradle", commands: &["gradle"], arguments: &["-version"] },
    Tool { id: "rust", commands: &["rustc"], arguments: &["--version"] },
    Tool { id: "mysql", commands: &["mysql"], arguments: &["--version"] },
    Tool { id: "postgresql", commands: &["psql"], arguments: &["--version"] },
    Tool { id: "redis", commands: &["redis-server"], arguments: &["--version"] },
    Tool { id: "docker", commands: &["docker"], arguments: &["--version"] },
    Tool { id: "vscode", commands: &["code"], arguments: &["--version"] },
    Tool { id: "intellij", commands: &[], arguments: &[] },
];

fn tool(id: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|item| item.id == id)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_millis() as i64).unwrap_or(0)
}

fn is_executable(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file() && metadata.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn trusted(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let home_text = home().to_string_lossy().into_owned();
    const SYSTEM: &[&str] = &[
        "/usr/bin/", "/bin/", "/usr/sbin/", "/sbin/", "/opt/homebrew/", "/usr/local/",
        "/Library/Java/", "/Library/Frameworks/", "/Applications/", "/opt/anaconda3/", "/opt/miniconda3/",
        "/System/Volumes/", "/private/",
    ];
    if SYSTEM.iter().any(|prefix| text.starts_with(prefix)) {
        return true;
    }
    if !home_text.is_empty() && text.starts_with(&home_text) {
        return text.contains("/.nvm/")
            || text.contains("/.pyenv/")
            || text.contains("/.cargo/")
            || text.contains("/.rustup/")
            || text.contains("/.volta/")
            || text.contains("/.asdf/")
            || text.contains("/.local/share/fnm/")
            || text.contains("/.local/share/uv/")
            || text.contains(".jdk/Contents/Home/")
            || text.contains("/jdk-")
            || text.contains("/Library/Java/")
            || text.contains("/Library/Application Support/com.yujian.devhub");
    }
    false
}

fn add_dir(dirs: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_dir() && !dirs.iter().any(|item| item == &path) {
        dirs.push(path);
    }
}

fn child_dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else { return Vec::new() };
    entries.flatten().filter_map(|entry| {
        let path = entry.path();
        if path.is_dir() { Some(path) } else { None }
    }).take(128).collect()
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            if dir.is_absolute() { add_dir(&mut dirs, dir); }
        }
    }
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin", "/usr/local/mysql/bin", "/usr/local/maven/bin", "/usr/local/gradle/bin", "/opt/anaconda3/bin", "/opt/miniconda3/bin"] {
        add_dir(&mut dirs, PathBuf::from(dir));
    }
    let home = home();
    for rel in [".cargo/bin", ".local/bin", ".volta/bin", ".asdf/shims", ".nvm/versions/node", ".pyenv/versions"] {
        let base = home.join(rel);
        if rel == ".nvm/versions/node" || rel == ".pyenv/versions" {
            for version in child_dirs(&base) { add_dir(&mut dirs, version.join("bin")); }
        } else {
            add_dir(&mut dirs, base);
        }
    }
    for fnm_root in [home.join("Library/Application Support/fnm/node-versions"), home.join(".local/share/fnm/node-versions")] {
        for version in child_dirs(&fnm_root) { add_dir(&mut dirs, version.join("installation/bin")); }
    }
    let uv_root = home.join(".local/share/uv/python");
    for python in child_dirs(&uv_root) { add_dir(&mut dirs, python.join("bin")); }
    for jdk_root in [PathBuf::from("/Library/Java/JavaVirtualMachines"), home.join("Library/Java/JavaVirtualMachines")] {
        for jdk in child_dirs(&jdk_root) { add_dir(&mut dirs, jdk.join("Contents/Home/bin")); }
    }
    if let Ok(entries) = fs::read_dir(&home) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("");
            if name.starts_with("jdk-") && path.is_dir() { add_dir(&mut dirs, path.join("Contents/Home/bin")); }
        }
    }
    dirs
}

fn app_paths(id: &str) -> Vec<PathBuf> {
    let home = home();
    let mut paths = Vec::new();
    match id {
        "vscode" => {
            paths.push(PathBuf::from("/Applications/Visual Studio Code.app"));
            paths.push(home.join("Applications/Visual Studio Code.app"));
        }
        "intellij" => {
            paths.push(PathBuf::from("/Applications/IntelliJ IDEA.app"));
            paths.push(PathBuf::from("/Applications/IntelliJ IDEA CE.app"));
            paths.push(home.join("Applications/IntelliJ IDEA.app"));
            paths.push(home.join("Applications/IntelliJ IDEA CE.app"));
        }
        "docker" => {
            paths.push(PathBuf::from("/Applications/Docker.app"));
            paths.push(home.join("Applications/Docker.app"));
        }
        _ => {}
    }
    paths
}

#[derive(Clone)]
struct Candidate {
    software_id: String,
    source: &'static str,
    managed_by: &'static str,
    kind: &'static str,
    path: PathBuf,
    command: Option<&'static str>,
    version_hint: Option<String>,
}

fn source_priority(source: &str) -> u8 {
    match source {
        "devhubV2" => 5,
        "devhubV1" => 4,
        "homebrewFormula" | "homebrewCask" => 3,
        "path" => 2,
        _ => 1,
    }
}

fn command_path(prefix: &Path, formula: &str, version: &str, commands: &[&str]) -> Option<PathBuf> {
    for command in commands {
        let cellar = prefix.join("Cellar").join(formula).join(version).join("bin").join(command);
        if is_executable(&cellar) { return Some(cellar); }
        let opt = prefix.join("opt").join(formula).join("bin").join(command);
        if is_executable(&opt) { return Some(opt); }
    }
    // Python versioned formulae may only expose python3.13, not python3.
    let bin = prefix.join("Cellar").join(formula).join(version).join("bin");
    if let Ok(entries) = fs::read_dir(&bin) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("python3") && is_executable(&entry.path()) {
                return Some(entry.path());
            }
        }
    }
    None
}

fn software_for_formula(name: &str) -> Option<&'static str> {
    if name == "git" { return Some("git"); }
    if name == "node" || name.starts_with("node@") { return Some("node"); }
    if name == "python" || name.starts_with("python@") { return Some("python"); }
    if name == "openjdk" || name.starts_with("openjdk@") { return Some("jdk"); }
    if name == "go" { return Some("go"); }
    if name == "maven" { return Some("maven"); }
    if name == "gradle" { return Some("gradle"); }
    if name == "rust" { return Some("rust"); }
    if name == "mysql" || name.starts_with("mysql@") { return Some("mysql"); }
    if name == "postgresql" || name.starts_with("postgresql@") { return Some("postgresql"); }
    if name == "redis" { return Some("redis"); }
    if name == "docker" { return Some("docker"); }
    None
}

fn software_for_cask(token: &str) -> Option<&'static str> {
    if token == "visual-studio-code" { return Some("vscode"); }
    if token == "intellij-idea" || token == "intellij-idea-ce" { return Some("intellij"); }
    if token == "docker" || token == "docker-desktop" { return Some("docker"); }
    None
}

fn locate_brew() -> Option<PathBuf> {
    for candidate in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
        let path = PathBuf::from(candidate);
        if is_executable(&path) { return Some(path); }
    }
    None
}

fn detect_homebrew(errors: &mut Vec<DetectionError>) -> (HomebrewStatus, Vec<Candidate>) {
    detect_homebrew_at(locate_brew(), errors)
}

fn detect_homebrew_at(brew: Option<PathBuf>, errors: &mut Vec<DetectionError>) -> (HomebrewStatus, Vec<Candidate>) {
    let Some(brew) = brew else {
        return (HomebrewStatus { available: false, prefix: None, version: None, error: None }, Vec::new());
    };
    let mut status = HomebrewStatus { available: true, prefix: None, version: None, error: None };
    let prefix = match safe_process::run(&brew, &["--prefix"], Duration::from_secs(10), 64 * 1024) {
        Ok(output) if output.status == 0 => PathBuf::from(output.stdout.trim()),
        _ => {
            status.error = Some("无法读取 Homebrew 前缀".into());
            errors.push(DetectionError { source: "homebrew".into(), message: "无法读取 Homebrew 前缀".into() });
            return (status, Vec::new());
        }
    };
    status.prefix = Some(prefix.to_string_lossy().into_owned());
    if let Ok(output) = safe_process::run(&brew, &["--version"], Duration::from_secs(10), 64 * 1024) {
        if output.status == 0 {
            status.version = output.stdout.lines().next().map(|line| line.trim().trim_start_matches("Homebrew ").to_string()).filter(|line| !line.is_empty());
        }
    }
    let info = match safe_process::run(&brew, &["info", "--json=v2", "--installed"], BREW_TIMEOUT, BREW_LIMIT) {
        Ok(output) if output.status == 0 => output.stdout,
        Ok(_) => {
            status.error = Some("Homebrew 已安装软件查询失败".into());
            errors.push(DetectionError { source: "homebrew".into(), message: "Homebrew 已安装软件查询失败".into() });
            return (status, Vec::new());
        }
        Err(error) => {
            status.error = Some(error.clone());
            errors.push(DetectionError { source: "homebrew".into(), message: error });
            return (status, Vec::new());
        }
    };
    let parsed: serde_json::Value = match serde_json::from_str(&info) {
        Ok(value) => value,
        Err(error) => {
            status.error = Some(format!("Homebrew JSON 无法解析：{error}"));
            errors.push(DetectionError { source: "homebrew".into(), message: "Homebrew JSON 无法解析".into() });
            return (status, Vec::new());
        }
    };
    let mut candidates = Vec::new();
    if let Some(formulae) = parsed.get("formulae").and_then(|value| value.as_array()) {
        for formula in formulae {
            let name = formula.get("name").and_then(|value| value.as_str()).unwrap_or("");
            let Some(id) = software_for_formula(name) else { continue };
            let Some(tool) = tool(id) else { continue };
            let Some(installed) = formula.get("installed").and_then(|value| value.as_array()) else { continue };
            for entry in installed {
                let version = entry.get("version").and_then(|value| value.as_str()).unwrap_or("");
                if version.is_empty() { continue; }
                let found = command_path(&prefix, name, version, tool.commands);
                let (path, command) = match found {
                    Some(path) => (path, tool.commands.first().copied()),
                    None => (prefix.join("opt").join(name), None),
                };
                candidates.push(Candidate {
                    software_id: id.into(),
                    source: "homebrewFormula",
                    managed_by: "homebrew",
                    kind: if command.is_some() { "cli" } else { "package" },
                    path,
                    command,
                    version_hint: Some(version.to_string()),
                });
            }
        }
    }
    if let Some(casks) = parsed.get("casks").and_then(|value| value.as_array()) {
        for cask in casks {
            let token = cask.get("token").and_then(|value| value.as_str()).unwrap_or("");
            let Some(id) = software_for_cask(token) else { continue };
            let version = cask.get("installed").and_then(|value| value.as_str()).map(str::to_string)
                .or_else(|| cask.get("installed").and_then(|value| value.as_array()).and_then(|values| values.first()).and_then(|value| value.as_str()).map(str::to_string));
            for path in app_paths(id) {
                if path.is_dir() {
                    candidates.push(Candidate {
                        software_id: id.into(),
                        source: "homebrewCask",
                        managed_by: "homebrew",
                        kind: "app",
                        path,
                        command: None,
                        version_hint: version.clone(),
                    });
                }
            }
        }
    }
    (status, candidates)
}

fn collect_path_candidates(tools: &[Tool]) -> Vec<Candidate> {
    let dirs = search_dirs();
    let mut candidates = Vec::new();
    for tool in tools {
        for command in tool.commands {
            for dir in &dirs {
                let path = dir.join(command);
                // /usr/bin/java is an Apple launcher, not an independent JDK installation.
                if tool.id == "jdk" && path == Path::new("/usr/bin/java") { continue; }
                if is_executable(&path) {
                    candidates.push(Candidate {
                        software_id: tool.id.into(),
                        source: "path",
                        managed_by: "system",
                        kind: "cli",
                        path,
                        command: Some(command),
                        version_hint: None,
                    });
                }
            }
        }
    }
    candidates
}

fn collect_app_candidates() -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for id in ["vscode", "intellij", "docker"] {
        for path in app_paths(id) {
            if path.is_dir() {
                candidates.push(Candidate {
                    software_id: id.into(),
                    source: "officialApp",
                    managed_by: "system",
                    kind: "app",
                    path,
                    command: None,
                    version_hint: None,
                });
            }
        }
    }
    candidates
}

fn find_managed_command(root: &Path, commands: &[&str], depth: usize) -> Option<PathBuf> {
    for command in commands {
        let direct = root.join("bin").join(command);
        if is_executable(&direct) { return Some(direct); }
        let jdk = root.join("Contents/Home/bin").join(command);
        if is_executable(&jdk) { return Some(jdk); }
    }
    if depth == 0 { return None; }
    for child in child_dirs(root) {
        if let Some(found) = find_managed_command(&child, commands, depth - 1) { return Some(found); }
    }
    None
}

fn collect_managed_candidates(root: &Path, source: &'static str, managed_by: &'static str, errors: &mut Vec<DetectionError>) -> Vec<Candidate> {
    let versions = root.join("versions");
    if !versions.is_dir() { return Vec::new(); }
    let mut candidates = Vec::new();
    let Ok(entries) = fs::read_dir(&versions) else {
        errors.push(DetectionError { source: source.into(), message: format!("无法读取 {}（只读检测）", versions.display()) });
        return candidates;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() { continue; }
        let folder = entry.file_name().to_string_lossy().into_owned();
        let id = match folder.as_str() {
            "node" => "node",
            "python" | "python3" => "python",
            "jdk" | "java" => "jdk",
            "go" => "go",
            "maven" => "maven",
            "gradle" => "gradle",
            "rust" => "rust",
            _ => continue,
        };
        let Some(tool) = tool(id) else { continue };
        for version_dir in child_dirs(&dir) {
            if let Some(path) = find_managed_command(&version_dir, tool.commands, 4) {
                candidates.push(Candidate {
                    software_id: id.into(),
                    source,
                    managed_by,
                    kind: "cli",
                    path,
                    command: tool.commands.first().copied(),
                    version_hint: None,
                });
            }
        }
    }
    candidates
}

fn probe(mut candidate: Candidate, sources: Vec<String>, last_detected_at: i64) -> SoftwareInstance {
    let mut version = candidate.version_hint.clone();
    let mut health = "unverified";
    let mut detail = None;
    let executable = is_executable(&candidate.path);
    if candidate.kind == "app" {
        match app_version(&candidate.path) {
            Ok(value) => { version = Some(value); health = "ok"; }
            Err(error) => { detail = Some(error); }
        }
    } else if candidate.command.is_none() || !executable {
        detail = Some("已安装，但未定位到可执行文件".into());
    } else if !trusted(&candidate.path) {
        detail = Some("可执行文件不在受信任的检测目录，未运行版本探针".into());
    } else {
        let args = tool(&candidate.software_id).map(|item| item.arguments).unwrap_or(&[]);
        match safe_process::run(&candidate.path, args, PROBE_TIMEOUT, PROBE_LIMIT) {
            Ok(output) => {
                let text = format!("{}\n{}", output.stdout, output.stderr);
                if output.status != 0 {
                    health = "broken";
                    detail = Some("版本探针返回非零状态".into());
                } else if let Some(parsed) = parse_version(&candidate.software_id, &text) {
                    version = Some(parsed);
                    health = "ok";
                } else {
                    detail = Some("命令可执行，但无法解析版本".into());
                }
            }
            Err(error) => { detail = Some(error); }
        }
    }
    if health != "ok" && version.is_none() {
        candidate.version_hint = None;
    }
    SoftwareInstance {
        software_id: candidate.software_id,
        version,
        source: candidate.source.into(),
        sources,
        path: candidate.path.to_string_lossy().into_owned(),
        kind: candidate.kind.into(),
        executable,
        managed_by: if candidate.path.to_string_lossy().contains("/fnm/node-versions/") { "fnm" } else if candidate.path.to_string_lossy().contains("/uv/python/") { "uv" } else { candidate.managed_by }.into(),
        health: health.into(),
        detail,
        last_detected_at,
    }
}

fn app_version(app: &Path) -> Result<String, String> {
    let info = app.join("Contents/Info.plist");
    if !info.is_file() { return Err("应用包缺少 Info.plist".into()); }
    let info_arg = info.to_string_lossy().into_owned();
    let output = safe_process::run(
        Path::new("/usr/libexec/PlistBuddy"),
        &["-c", "Print :CFBundleShortVersionString", &info_arg],
        Duration::from_secs(5),
        64 * 1024,
    )?;
    if output.status != 0 { return Err("无法读取应用版本".into()); }
    let value = output.stdout.trim();
    if value.is_empty() { return Err("应用版本为空".into()); }
    Ok(value.to_string())
}

fn parse_version(id: &str, text: &str) -> Option<String> {
    let first = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let after = |prefix: &str| first.strip_prefix(prefix).map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);
    match id {
        "node" => after("v").or_else(|| first.split_whitespace().next().map(|value| value.trim_start_matches('v').to_string())),
        "python" => after("Python "),
        "git" => after("git version ").map(|value| value.split_whitespace().next().unwrap_or(&value).to_string()),
        "jdk" => {
            if let Some(start) = text.find('"') {
                let rest = &text[start + 1..];
                let end = rest.find('"')?;
                return Some(rest[..end].to_string());
            }
            after("java version ").or_else(|| after("openjdk version "))
        }
        "go" => text.split_whitespace().find(|token| token.starts_with("go") && token[2..].starts_with(|c: char| c.is_ascii_digit())).map(|token| token[2..].to_string()),
        "maven" => after("Apache Maven ").map(|value| value.split_whitespace().next().unwrap_or("").to_string()).filter(|value| !value.is_empty()),
        "gradle" => after("Gradle ").map(|value| value.split_whitespace().next().unwrap_or("").to_string()).filter(|value| !value.is_empty()),
        "rust" => after("rustc ").map(|value| value.split_whitespace().next().unwrap_or("").to_string()).filter(|value| !value.is_empty()),
        "mysql" => first.split("Ver ").nth(1).map(|value| value.split_whitespace().next().unwrap_or(value).to_string()),
        "postgresql" => text.find("(PostgreSQL) ").map(|index| text[index + "(PostgreSQL) ".len()..].split_whitespace().next().unwrap_or("").to_string()),
        "redis" => text.split("v=").nth(1).map(|value| value.split_whitespace().next().unwrap_or(value).to_string()),
        "docker" => after("Docker version "),
        "vscode" => first.split_whitespace().next().map(str::to_string),
        _ => first.split_whitespace().find(|token| token.chars().any(|c| c.is_ascii_digit())).map(str::to_string),
    }
}

fn merge_candidates(candidates: Vec<Candidate>) -> HashMap<(String, String), (Candidate, Vec<String>)> {
    let mut merged: HashMap<(String, String), (Candidate, Vec<String>)> = HashMap::new();
    for candidate in candidates {
        let canonical = fs::canonicalize(&candidate.path).unwrap_or_else(|_| candidate.path.clone());
        let key = (candidate.software_id.clone(), canonical.to_string_lossy().into_owned());
        if let Some((existing, sources)) = merged.get_mut(&key) {
            if !sources.iter().any(|source| source == candidate.source) {
                sources.push(candidate.source.into());
            }
            if source_priority(candidate.source) > source_priority(existing.source) {
                *existing = candidate;
            }
        } else {
            merged.insert(key, (candidate.clone(), vec![candidate.source.into()]));
        }
    }
    merged
}

pub fn scan() -> DetectionSnapshot {
    let started = Instant::now();
    let mut errors = Vec::new();
    let mut candidates = Vec::new();
    candidates.extend(collect_path_candidates(TOOLS));
    candidates.extend(collect_app_candidates());
    let (homebrew, brew_candidates) = detect_homebrew(&mut errors);
    candidates.extend(brew_candidates);
    let home = home();
    for (profile, source) in [("com.yujian.devhub", "devhubV1"), ("com.yujian.devhub.validation", "devhubV1"), ("com.yujian.devhub.v2", "devhubV2")] {
        let root = home.join("Library/Application Support").join(profile).join("storage-v1");
        if root.is_dir() {
            candidates.extend(collect_managed_candidates(&root, source, "devhub", &mut errors));
        }
    }
    let mut instances = Vec::new();
    for (_, (candidate, sources)) in merge_candidates(candidates) {
        instances.push(probe(candidate, sources, now_ms()));
    }
    instances.sort_by(|left, right| left.software_id.cmp(&right.software_id).then_with(|| right.version.cmp(&left.version)));
    DetectionSnapshot {
        schema_version: 1,
        refreshed_at: Some(now_ms()),
        stale: false,
        duration_ms: started.elapsed().as_millis() as u64,
        homebrew,
        instances,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_version_formats() {
        assert_eq!(parse_version("node", "v26.13.0").as_deref(), Some("26.13.0"));
        assert_eq!(parse_version("python", "Python 3.14.6").as_deref(), Some("3.14.6"));
        assert_eq!(parse_version("git", "git version 2.50.1 (Apple Git-155)").as_deref(), Some("2.50.1"));
        assert_eq!(parse_version("jdk", "openjdk version \"17.0.2\" 2022-01-18").as_deref(), Some("17.0.2"));
        assert_eq!(parse_version("go", "go version go1.27.3 darwin/arm64").as_deref(), Some("1.27.3"));
        assert_eq!(parse_version("maven", "Apache Maven 3.9.16 (abc)").as_deref(), Some("3.9.16"));
        assert_eq!(parse_version("gradle", "Gradle 9.9.0").as_deref(), Some("9.9.0"));
        assert_eq!(parse_version("rust", "rustc 1.99.1 (58cf8f9c3 2026-10-13)").as_deref(), Some("1.99.1"));
    }

    #[test]
    fn anchors_do_not_live_in_untrusted_temp_dirs() {
        assert!(trusted(Path::new("/usr/bin/git")));
        assert!(trusted(Path::new("/opt/homebrew/bin/brew")));
        assert!(!trusted(Path::new("/tmp/evil/node")));
        assert!(trusted(&home().join(".local/share/fnm/node-versions/v22.14.0/installation/bin/node")));
        assert!(trusted(&home().join(".local/share/uv/python/cpython-3.12-macos-aarch64-none/bin/python3")));
    }

    #[test]
    fn missing_homebrew_is_a_normal_state() {
        let mut errors = Vec::new();
        let (status, candidates) = detect_homebrew_at(None, &mut errors);
        assert!(!status.available);
        assert!(candidates.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn maps_homebrew_formulae_and_casks_to_catalog_ids() {
        assert_eq!(software_for_formula("python@3.13"), Some("python"));
        assert_eq!(software_for_formula("node@22"), Some("node"));
        assert_eq!(software_for_formula("openjdk@21"), Some("jdk"));
        assert_eq!(software_for_formula("maven"), Some("maven"));
        assert_eq!(software_for_formula("ncmdump"), None);
        assert_eq!(software_for_cask("visual-studio-code"), Some("vscode"));
        assert_eq!(software_for_cask("intellij-idea"), Some("intellij"));
        assert_eq!(software_for_cask("unknown-cask"), None);
    }

    #[test]
    #[ignore]
    fn live_detection_report() {
        let snapshot = scan();
        println!("{}", serde_json::to_string_pretty(&snapshot).expect("snapshot json"));
    }

    #[test]
    fn merge_prefers_devhub_and_keeps_all_sources() {
        let base = Candidate { software_id: "node".into(), source: "path", managed_by: "system", kind: "cli", path: PathBuf::from("/usr/local/bin/node"), command: Some("node"), version_hint: None };
        let managed = Candidate { source: "devhubV1", managed_by: "devhub", ..base.clone() };
        let merged = merge_candidates(vec![base, managed]);
        let (candidate, sources) = merged.values().next().expect("merged");
        assert_eq!(candidate.managed_by, "devhub");
        assert!(sources.iter().any(|source| source == "devhubV1"));
        assert!(sources.iter().any(|source| source == "path"));
    }
}
