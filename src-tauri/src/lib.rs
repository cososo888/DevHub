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
fn get_software_plans() -> Vec<homebrew::PackagePlan> {
    homebrew::plans()
}

#[tauri::command]
fn open_homebrew_site() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("https://brew.sh")
        .status()
        .map(|_| ())
        .map_err(|error| format!("无法打开 Homebrew 页面：{error}"))
}

fn is_brew_managed(instance: &crate::detection::SoftwareInstance) -> bool {
    instance.source == "homebrewFormula"
        || instance.source == "homebrewCask"
        || instance.path.contains("/Cellar/")
        || instance.path.contains("/Caskroom/")
        || instance.path.contains("/opt/homebrew/")
}

fn service_note(software_id: &str) -> &'static str {
    match software_id {
        "mysql" | "postgresql" | "redis" => "软件包已安装；数据库服务尚未自动启动",
        _ => "安装完成，已重新检测本机环境",
    }
}

#[tauri::command]
fn start_install(app: AppHandle, state: State<AppState>, software_id: String) -> Result<u64, String> {
    let plan = homebrew::plan(&software_id).ok_or_else(|| "该软件没有可靠的 Homebrew 安装命令".to_string())?;
    homebrew::probe().map_err(|_| "安装需要 Homebrew，请先准备 Homebrew 后重试".to_string())?;
    let snapshot = snapshot_clone(&state).ok_or_else(|| "尚未完成本机检测，请先刷新".to_string())?;
    if snapshot.instances.iter().any(|item| item.software_id == software_id) {
        return Err("本机已经检测到该软件，无需重复安装".into());
    }
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        emit_progress(&app, job_id, &software_id, &plan.package, "install", "installing", &format!("正在执行 {}", plan.install_command));
        let output = match homebrew::install_package(&plan) {
            Ok(value) => value,
            Err(error) => {
                let snapshot = detect_now(&app, &state);
                let found = snapshot.instances.iter().find(|item| item.software_id == software_id);
                emit_finished(&app, job_id, &software_id, &plan.package, "install", false, &format!("安装失败：{error}"), &error, Some(Verification { verified: false, instance_path: found.map(|item| item.path.clone()), version: found.and_then(|item| item.version.clone()), health: found.map(|item| item.health.clone()), system_git_available: None }));
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, &plan.package, "install", "verifying", "命令结束，正在确认实际安装结果…");
        let snapshot = detect_now(&app, &state);
        let found = snapshot.instances.iter().find(|item| item.software_id == software_id);
        let package_present = homebrew::package_installed(&plan).unwrap_or(false);
        if !package_present {
            emit_finished(&app, job_id, &software_id, &plan.package, "install", false, "安装命令结束，但未能确认 Homebrew 包已安装", &output, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: None }));
            return;
        }
        let message = if found.is_some() { service_note(&software_id) } else { "Homebrew 包已安装；当前未检测到可执行入口，可能需要重新打开终端或该服务尚未启动" };
        emit_finished(&app, job_id, &software_id, &plan.package, "install", true, message, &output, Some(Verification { verified: found.is_some(), instance_path: found.map(|item| item.path.clone()), version: found.and_then(|item| item.version.clone()), health: found.map(|item| item.health.clone()), system_git_available: None }));
    });
    Ok(job_id)
}

#[tauri::command]
fn start_uninstall(app: AppHandle, state: State<AppState>, software_id: String) -> Result<u64, String> {
    let plan = homebrew::plan(&software_id).ok_or_else(|| "该软件没有可靠的 Homebrew 卸载命令".to_string())?;
    let snapshot = snapshot_clone(&state).ok_or_else(|| "尚未完成本机检测，请先刷新".to_string())?;
    let managed = snapshot.instances.iter().filter(|item| item.software_id == software_id && is_brew_managed(item)).collect::<Vec<_>>();
    if managed.is_empty() {
        return Err("该安装不是 Homebrew 管理，DevHub 不会直接删除文件".into());
    }
    if !homebrew::package_installed(&plan).unwrap_or(false) {
        return Err("Homebrew 未记录该软件包，无法安全卸载".into());
    }
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let history_version = managed.first().and_then(|item| item.version.clone()).unwrap_or_else(|| "unknown".into());
    let history_path = managed.first().map(|item| item.path.clone());
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        emit_progress(&app, job_id, &software_id, &plan.package, "uninstall", "uninstalling", &format!("正在执行 {}", plan.uninstall_command));
        let output = match homebrew::uninstall_package(&plan) {
            Ok(value) => value,
            Err(error) => {
                let snapshot = detect_now(&app, &state);
                let found = snapshot.instances.iter().find(|item| item.software_id == software_id);
                emit_finished(&app, job_id, &software_id, &plan.package, "uninstall", false, &format!("卸载失败：{error}"), &error, Some(Verification { verified: false, instance_path: found.map(|item| item.path.clone()), version: found.and_then(|item| item.version.clone()), health: found.map(|item| item.health.clone()), system_git_available: None }));
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, &plan.package, "uninstall", "verifying", "命令结束，正在确认软件包是否已移除…");
        let snapshot = detect_now(&app, &state);
        let package_present = homebrew::package_installed(&plan).unwrap_or(true);
        let still_managed = snapshot.instances.iter().any(|item| item.software_id == software_id && is_brew_managed(item));
        if package_present || still_managed {
            emit_finished(&app, job_id, &software_id, &plan.package, "uninstall", false, "卸载命令结束，但仍检测到 Homebrew 管理的安装", &output, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: None }));
            return;
        }
        if let Ok(connection) = state.db.lock() {
            let _ = storage::record_uninstall(&connection, &software_id, &history_version, "homebrew", history_path.as_deref(), now_ms());
        }
        emit_finished(&app, job_id, &software_id, &plan.package, "uninstall", true, "Homebrew 包已移除；系统或其他来源的安装不会被改动", &output, Some(Verification { verified: true, instance_path: None, version: None, health: None, system_git_available: None }));
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
            get_software_plans,
            refresh_detection,
            open_homebrew_site,
            start_install,
            start_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevHub V2");
}
