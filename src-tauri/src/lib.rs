mod detection;
mod desktop;
mod homebrew;
mod notifications;
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
    kind: String,
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
    kind: String,
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

fn emit_progress(app: &AppHandle, job_id: u64, software_id: &str, kind: &str, action: &str, stage: &str, message: &str) {
    let _ = app.emit("installation://progress", InstallProgress {
        job_id, software_id: software_id.into(), kind: kind.into(), version: software_id.into(), action: action.into(),
        stage: stage.into(), message: message.into(), percent: None,
    });
}

fn emit_finished(app: &AppHandle, job_id: u64, software_id: &str, kind: &str, action: &str, success: bool, message: &str, output: &str, verification: Option<Verification>) {
    let verified_install = action == "install" && success && verification.as_ref().is_some_and(|result| result.verified);
    let _ = app.emit("installation://finished", InstallFinished {
        job_id, software_id: software_id.into(), kind: kind.into(), version: software_id.into(), action: action.into(),
        success, message: message.into(), output: truncate_output(output), verification,
    });
    if verified_install { desktop::installation_complete(app, software_id); }
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
async fn get_installed_packages() -> Result<homebrew::InstalledPackages, String> {
    tauri::async_runtime::spawn_blocking(homebrew::installed_packages).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn search_homebrew(query: String) -> Result<Vec<homebrew::HomebrewResult>, String> {
    tauri::async_runtime::spawn_blocking(move || homebrew::search(&query)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_homebrew_site() -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg("https://brew.sh/")
        .status()
        .map_err(|error| format!("无法打开 Homebrew 页面：{error}"))?;
    if status.success() { Ok(()) } else { Err(format!("浏览器未打开 Homebrew 页面（{status}）")) }
}

fn service_note(package: &str) -> &'static str {
    match package {
        "mysql" | "postgresql@17" | "redis" | "memcached" => "软件包已安装；数据库/缓存服务尚未自动启动",
        _ => "Homebrew 已确认安装完成",
    }
}

#[tauri::command]
fn start_package_install(app: AppHandle, state: State<AppState>, package: String, kind: String) -> Result<u64, String> {
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        let command = if kind == "cask" { format!("brew install --cask {package}") } else { format!("brew install --formula {package}") };
        emit_progress(&app, job_id, &package, &kind, "install", "installing", &format!("正在执行 {command}"));
        let output = match homebrew::install_package(&package, &kind) {
            Ok(value) => value,
            Err(error) => {
                let _ = detect_now(&app, &state);
                emit_finished(&app, job_id, &package, &kind, "install", false, "安装命令未完成，请查看技术日志", &error, None);
                return;
            }
        };
        emit_progress(&app, job_id, &package, &kind, "install", "verifying", "命令结束，正在确认 Homebrew 安装状态…");
        detect_now(&app, &state);
        let package_present = match homebrew::package_installed_verified(&package, &kind) {
            Ok(value) => value,
            Err(error) => {
                emit_finished(&app, job_id, &package, &kind, "install", false, "安装命令结束，但无法确认 Homebrew 状态", &format!("{output}\n\n[安装结果验证]\n{error}"), None);
                return;
            }
        };
        if !package_present {
            emit_finished(&app, job_id, &package, &kind, "install", false, "安装命令结束，但 Homebrew 未确认该软件包已安装", &output, None);
            return;
        }
        emit_finished(&app, job_id, &package, &kind, "install", true, service_note(&package), &output, Some(Verification {
            verified: true, instance_path: None, version: None, health: None, system_git_available: None,
        }));
    });
    Ok(job_id)
}

#[tauri::command]
fn start_package_uninstall(app: AppHandle, state: State<AppState>, package: String, kind: String) -> Result<u64, String> {
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        let version = homebrew::installed_packages().ok().and_then(|packages| {
            let rows = if kind == "cask" { packages.casks } else { packages.formulae };
            rows.into_iter().find(|row| row.package == package).map(|row| row.version)
        }).unwrap_or_else(|| "unknown".into());
        let command = if kind == "cask" { format!("brew uninstall --cask {package}") } else { format!("brew uninstall --formula {package}") };
        emit_progress(&app, job_id, &package, &kind, "uninstall", "uninstalling", &format!("正在执行 {command}"));
        if let Err(error) = homebrew::uninstall_package(&package, &kind) {
            let _ = detect_now(&app, &state);
            emit_finished(&app, job_id, &package, &kind, "uninstall", false, &format!("卸载失败：{error}"), &error, None);
            return;
        }
        emit_progress(&app, job_id, &package, &kind, "uninstall", "verifying", "命令结束，正在确认 Homebrew 包已移除…");
        let _ = detect_now(&app, &state);
        match homebrew::package_installed(&package, &kind) {
            Ok(false) => (),
            Ok(true) => {
                emit_finished(&app, job_id, &package, &kind, "uninstall", false, "卸载命令结束，但 Homebrew 仍报告该包已安装", "", None);
                return;
            }
            Err(error) => {
                emit_finished(&app, job_id, &package, &kind, "uninstall", false, "卸载后的软件状态尚未确认，请刷新检测", &error, None);
                return;
            }
        }
        if let Ok(connection) = state.db.lock() {
            let _ = storage::record_uninstall(&connection, &package, &version, "homebrew", None, now_ms());
        }
        emit_finished(&app, job_id, &package, &kind, "uninstall", true, "Homebrew 包已移除；系统或其他来源的安装不会被改动", "", None);
    });
    Ok(job_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .on_window_event(desktop::on_window_event)
        .setup(|app| {
            desktop::setup(app)?;
            let dir = app.path().app_data_dir().map_err(|error| std::io::Error::other(error.to_string()))?;
            let state = AppState::new(dir).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            notifications::get_notification_status,
            notifications::request_notification_permission,
            notifications::send_test_notification,
            notifications::open_notification_settings,
            get_app_status,
            get_detection_snapshot,
            get_installed_packages,
            search_homebrew,
            refresh_detection,
            open_homebrew_site,
            start_package_install,
            start_package_uninstall
        ])
        .build(tauri::generate_context!())
        .expect("error while building DevHub V2");
    app.run(|handle, event| desktop::on_run_event(handle, &event));
}
