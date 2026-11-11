mod detection;
mod homebrew;
mod process;
mod storage;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use detection::DetectionSnapshot;
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    phase: u8,
    product: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallProgress {
    job_id: u64,
    software_id: String,
    version: String,
    action: String,
    stage: String,
    message: String,
    percent: Option<u8>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Verification {
    verified: bool,
    instance_path: Option<String>,
    version: Option<String>,
    health: Option<String>,
    system_git_available: Option<bool>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallFinished {
    job_id: u64,
    software_id: String,
    version: String,
    action: String,
    success: bool,
    message: String,
    output: String,
    verification: Option<Verification>,
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    snapshot: Arc<Mutex<Option<DetectionSnapshot>>>,
    refreshing: Arc<AtomicBool>,
    install_busy: Arc<AtomicBool>,
    job_seq: Arc<AtomicU64>,
}

impl AppState {
    fn new(dir: std::path::PathBuf) -> Result<Self, String> {
        let connection = storage::open(&dir)?;
        let snapshot = storage::load(&connection);
        Ok(Self {
            db: Arc::new(Mutex::new(connection)),
            snapshot: Arc::new(Mutex::new(snapshot)),
            refreshing: Arc::new(AtomicBool::new(false)),
            install_busy: Arc::new(AtomicBool::new(false)),
            job_seq: Arc::new(AtomicU64::new(0)),
        })
    }
}

struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_millis() as i64).unwrap_or(0)
}

fn truncate_output(text: &str) -> String {
    const LIMIT: usize = 160 * 1024;
    if text.len() <= LIMIT { return text.to_string(); }
    let mut end = LIMIT;
    while !text.is_char_boundary(end) { end -= 1; }
    format!("{}\n…（输出已截断，共 {} 字节）", &text[..end], text.len())
}

fn emit_progress(app: &AppHandle, job_id: u64, software_id: &str, version: &str, action: &str, stage: &str, message: &str) {
    let _ = app.emit("installation://progress", InstallProgress {
        job_id, software_id: software_id.into(), version: version.into(), action: action.into(),
        stage: stage.into(), message: message.into(), percent: None,
    });
}

fn emit_finished(app: &AppHandle, job_id: u64, software_id: &str, version: &str, action: &str, success: bool, message: &str, output: &str, verification: Option<Verification>) {
    let _ = app.emit("installation://finished", InstallFinished {
        job_id, software_id: software_id.into(), version: version.into(), action: action.into(),
        success, message: message.into(), output: truncate_output(output), verification,
    });
}

fn publish_snapshot(app: &AppHandle, state: &AppState, snapshot: &mut DetectionSnapshot) {
    if let Ok(connection) = state.db.lock() {
        if let Err(error) = storage::save(&connection, snapshot) {
            snapshot.errors.push(detection::DetectionError { source: "cache".into(), message: error });
        }
    }
    if let Ok(mut cached) = state.snapshot.lock() { *cached = Some(snapshot.clone()); }
    let _ = app.emit("detection://updated", snapshot.clone());
}

fn detect_now(app: &AppHandle, state: &AppState) -> DetectionSnapshot {
    let result = std::panic::catch_unwind(detection::scan);
    let failed = result.is_err();
    let mut snapshot = match result { Ok(value) => value, Err(_) => DetectionSnapshot::failed("检测线程异常".into()) };
    if !failed { snapshot.stale = false; }
    publish_snapshot(app, state, &mut snapshot);
    snapshot
}

fn snapshot_clone(state: &AppState) -> Option<DetectionSnapshot> {
    state.snapshot.lock().ok().and_then(|value| value.clone())
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    AppStatus { phase: 3, product: "DevHub V2" }
}

#[tauri::command]
fn get_detection_snapshot(state: State<AppState>) -> Option<DetectionSnapshot> {
    snapshot_clone(&state)
}

#[tauri::command]
fn refresh_detection(app: AppHandle, state: State<AppState>) -> bool {
    if state.refreshing.swap(true, Ordering::SeqCst) { return false; }
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = detect_now(&app, &state);
        state.refreshing.store(false, Ordering::SeqCst);
    });
    true
}

#[tauri::command]
fn get_installed_packages() -> Result<homebrew::InstalledPackages, String> {
    homebrew::installed_packages()
}

#[tauri::command]
fn search_homebrew(query: String) -> Result<Vec<homebrew::HomebrewResult>, String> {
    homebrew::search(&query)
}

#[tauri::command]
fn open_homebrew_site() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("https://brew.sh")
        .status()
        .map(|_| ())
        .map_err(|error| format!("无法打开 Homebrew 页面：{error}"))
}

fn service_note(package: &str) -> &'static str {
    match package {
        "mysql" | "postgresql@17" | "redis" => "软件包已安装；数据库/缓存服务尚未自动启动",
        _ => "安装完成，正在刷新本机环境",
    }
}

#[tauri::command]
fn start_package_install(app: AppHandle, state: State<AppState>, package: String, kind: String) -> Result<u64, String> {
    homebrew::probe().map_err(|_| "安装需要 Homebrew，请先准备 Homebrew 后重试".to_string())?;
    if !homebrew::package_exists(&package, &kind)? {
        return Err("Homebrew 中找不到该软件包".into());
    }
    if homebrew::package_installed(&package, &kind)? {
        return Err("Homebrew 已经安装该软件包".into());
    }
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        let command = if kind == "cask" { format!("brew install --cask {package}") } else { format!("brew install {package}") };
        emit_progress(&app, job_id, &package, &package, "install", "installing", &format!("正在执行 {command}"));
        let output = match homebrew::install_package(&package, &kind) {
            Ok(value) => value,
            Err(error) => {
                let _ = detect_now(&app, &state);
                emit_finished(&app, job_id, &package, &package, "install", false, &format!("安装失败：{error}"), &error, None);
                return;
            }
        };
        emit_progress(&app, job_id, &package, &package, "install", "verifying", "命令结束，正在确认 Homebrew 安装状态…");
        let snapshot = detect_now(&app, &state);
        let package_present = homebrew::package_installed(&package, &kind).unwrap_or(false);
        if !package_present {
            emit_finished(&app, job_id, &package, &package, "install", false, "安装命令结束，但 Homebrew 未确认该软件包已安装", &output, None);
            return;
        }
        let found = snapshot.instances.iter().find(|item| item.software_id == package || item.path.contains(&package));
        let message = if found.is_some() { service_note(&package) } else { "Homebrew 包已安装；当前未检测到可执行入口，可能需要重新打开终端" };
        emit_finished(&app, job_id, &package, &package, "install", true, message, &output, Some(Verification { verified: true, instance_path: found.map(|item| item.path.clone()), version: found.and_then(|item| item.version.clone()), health: found.map(|item| item.health.clone()), system_git_available: None }));
    });
    Ok(job_id)
}

#[tauri::command]
fn start_package_uninstall(app: AppHandle, state: State<AppState>, package: String, kind: String) -> Result<u64, String> {
    if !homebrew::package_installed(&package, &kind)? {
        return Err("只有 Homebrew 管理的软件包才能从 DevHub 卸载".into());
    }
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let version = homebrew::installed_packages().ok()
        .and_then(|state| state.formulae.into_iter().chain(state.casks).find(|item| item.package == package).map(|item| item.version))
        .unwrap_or_else(|| "unknown".into());
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        let command = if kind == "cask" { format!("brew uninstall --cask {package}") } else { format!("brew uninstall {package}") };
        emit_progress(&app, job_id, &package, &package, "uninstall", "uninstalling", &format!("正在执行 {command}"));
        if let Err(error) = homebrew::uninstall_package(&package, &kind) {
            let _ = detect_now(&app, &state);
            emit_finished(&app, job_id, &package, &package, "uninstall", false, &format!("卸载失败：{error}"), &error, None);
            return;
        }
        emit_progress(&app, job_id, &package, &package, "uninstall", "verifying", "命令结束，正在确认 Homebrew 包已移除…");
        let _ = detect_now(&app, &state);
        if homebrew::package_installed(&package, &kind).unwrap_or(true) {
            emit_finished(&app, job_id, &package, &package, "uninstall", false, "卸载命令结束，但 Homebrew 仍报告该包已安装", "", None);
            return;
        }
        if let Ok(connection) = state.db.lock() {
            let _ = storage::record_uninstall(&connection, &package, &version, "homebrew", None, now_ms());
        }
        emit_finished(&app, job_id, &package, &package, "uninstall", true, "Homebrew 包已移除；系统或其他来源的安装不会被改动", "", None);
    });
    Ok(job_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|error| std::io::Error::other(error.to_string()))?;
            let state = AppState::new(dir).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_detection_snapshot,
            get_installed_packages,
            search_homebrew,
            refresh_detection,
            open_homebrew_site,
            start_package_install,
            start_package_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevHub V2");
}
