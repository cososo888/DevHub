mod detection;
mod homebrew;
mod process;
mod storage;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use detection::DetectionSnapshot;
use homebrew::{InstallPreview, UninstallPreview};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    phase: u8,
    product: &'static str,
    detection: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallProgress {
    job_id: u64,
    software_id: String,
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
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
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

fn emit_progress(app: &AppHandle, job_id: u64, software_id: &str, action: &str, stage: &str, message: &str) {
    let _ = app.emit("installation://progress", InstallProgress {
        job_id,
        software_id: software_id.into(),
        action: action.into(),
        stage: stage.into(),
        message: message.into(),
        percent: None,
    });
}

fn emit_finished(app: &AppHandle, job_id: u64, software_id: &str, action: &str, success: bool, message: &str, output: &str, verification: Option<Verification>) {
    let _ = app.emit("installation://finished", InstallFinished {
        job_id,
        software_id: software_id.into(),
        action: action.into(),
        success,
        message: message.into(),
        output: truncate_output(output),
        verification,
    });
}

fn publish_snapshot(app: &AppHandle, state: &AppState, snapshot: &mut DetectionSnapshot) {
    if let Ok(connection) = state.db.lock() {
        if let Err(error) = storage::save(&connection, snapshot) {
            snapshot.errors.push(detection::DetectionError { source: "cache".into(), message: error });
        }
    }
    if let Ok(mut cached) = state.snapshot.lock() {
        *cached = Some(snapshot.clone());
    }
    let _ = app.emit("detection://updated", snapshot.clone());
}

fn detect_now(app: &AppHandle, state: &AppState) -> DetectionSnapshot {
    let result = std::panic::catch_unwind(detection::scan);
    let failed = result.is_err();
    let mut snapshot = match result {
        Ok(value) => value,
        Err(_) => DetectionSnapshot::failed("检测线程异常".into()),
    };
    if !failed { snapshot.stale = false; }
    publish_snapshot(app, state, &mut snapshot);
    snapshot
}

fn ownership_view(state: &AppState, software_id: &str) -> Option<homebrew::OwnershipView> {
    state.db.lock().ok().and_then(|connection| storage::active_for(&connection, software_id).map(|record| record.view()))
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    AppStatus { phase: 3, product: "DevHub V2", detection: "read-only" }
}

#[tauri::command]
fn get_detection_snapshot(state: State<AppState>) -> Option<DetectionSnapshot> {
    state.snapshot.lock().ok().and_then(|value| value.clone())
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
fn preview_homebrew_install(software_id: String, state: State<AppState>) -> InstallPreview {
    homebrew::preview_install(&software_id, ownership_view(&state, &software_id))
}

#[tauri::command]
fn preview_homebrew_uninstall(software_id: String, state: State<AppState>) -> UninstallPreview {
    homebrew::preview_uninstall(&software_id, ownership_view(&state, &software_id))
}

#[tauri::command]
fn start_homebrew_install(app: AppHandle, state: State<AppState>, software_id: String) -> Result<u64, String> {
    let Some((package, package_type)) = homebrew::package_for(&software_id) else {
        return Err("第三阶段先支持 Git 的 Homebrew 安装".into());
    };
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let ownership = ownership_view(&state, &software_id);
    let preview = homebrew::preview_install(&software_id, ownership);
    if !preview.can_install {
        state.install_busy.store(false, Ordering::SeqCst);
        return Err(preview.reason.unwrap_or_else(|| "当前不能安装".into()));
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        let started = now_ms();
        emit_progress(&app, job_id, &software_id, "install", "checking", "正在检查 Homebrew 与安装计划…");
        let brew = match homebrew::probe() {
            Ok(value) => value,
            Err(error) => {
                emit_finished(&app, job_id, &software_id, "install", false, &error, "", None);
                return;
            }
        };
        let ownership_id = match state.db.lock() {
            Ok(connection) => match storage::insert_pending(&connection, &software_id, package, package_type, false, started) {
                Ok(value) => Some(value),
                Err(error) => {
                    emit_finished(&app, job_id, &software_id, "install", false, &error, "", None);
                    return;
                }
            },
            Err(_) => {
                emit_finished(&app, job_id, &software_id, "install", false, "无法访问安装归属数据库", "", None);
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, "install", "installing", "正在执行 Homebrew 安装。Homebrew 不提供可靠百分比，因此显示不确定进度。");
        let output = match homebrew::install(&brew, &software_id) {
            Ok(value) => value,
            Err(error) => {
                if let (Some(id), Ok(connection)) = (ownership_id, state.db.lock()) {
                    let _ = storage::update_install(&connection, id, "failed", None, None, Some(&error), now_ms());
                }
                let snapshot = detect_now(&app, &state);
                let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
                emit_finished(&app, job_id, &software_id, "install", false, &format!("Homebrew 安装失败：{error}"), &error, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: Some(system) }));
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, "install", "verifying", "安装命令结束，正在重新检测并验证版本…");
        let snapshot = detect_now(&app, &state);
        let homebrew_git = snapshot.instances.iter().find(|item| item.software_id == "git" && (item.source == "homebrewFormula" || item.path.starts_with(&brew.prefix)));
        let verified = homebrew_git.filter(|item| item.health == "ok");
        match verified {
            Some(instance) => {
                if let (Some(id), Ok(connection)) = (ownership_id, state.db.lock()) {
                    if let Err(error) = storage::update_install(&connection, id, "success", Some(&instance.path), instance.version.as_deref(), None, now_ms()) {
                        emit_finished(&app, job_id, &software_id, "install", false, &error, &output, None);
                        return;
                    }
                }
                let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
                emit_finished(&app, job_id, &software_id, "install", true, "Homebrew Git 已安装并通过版本验证", &output, Some(Verification { verified: true, instance_path: Some(instance.path.clone()), version: instance.version.clone(), health: Some(instance.health.clone()), system_git_available: Some(system) }));
            }
            None => {
                if let (Some(id), Ok(connection)) = (ownership_id, state.db.lock()) {
                    let _ = storage::update_install(&connection, id, "verification_failed", homebrew_git.map(|item| item.path.as_str()), homebrew_git.and_then(|item| item.version.clone()).as_deref(), Some("安装后未检测到健康的 Homebrew Git"), now_ms());
                }
                emit_finished(&app, job_id, &software_id, "install", false, "Homebrew 命令成功，但安装后未验证到可用的 Homebrew Git；不会授予卸载归属", &output, None);
            }
        }
    });
    Ok(job_id)
}

#[tauri::command]
fn start_homebrew_uninstall(app: AppHandle, state: State<AppState>, software_id: String) -> Result<u64, String> {
    let Some((_package, _)) = homebrew::package_for(&software_id) else {
        return Err("第三阶段先支持 Git 的 Homebrew 卸载".into());
    };
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let ownership = ownership_view(&state, &software_id);
    let preview = homebrew::preview_uninstall(&software_id, ownership);
    if !preview.can_uninstall {
        state.install_busy.store(false, Ordering::SeqCst);
        return Err(preview.reason.unwrap_or_else(|| "当前不能卸载".into()));
    }
    let ownership = preview.ownership.clone();
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        emit_progress(&app, job_id, &software_id, "uninstall", "checking", "正在检查 Homebrew 依赖与卸载归属…");
        let brew = match homebrew::probe() {
            Ok(value) => value,
            Err(error) => { emit_finished(&app, job_id, &software_id, "uninstall", false, &error, "", None); return; }
        };
        let rechecked = homebrew::preview_uninstall(&software_id, ownership.clone());
        if !rechecked.can_uninstall {
            emit_finished(&app, job_id, &software_id, "uninstall", false, rechecked.reason.as_deref().unwrap_or("卸载前检查未通过"), "", None);
            return;
        }
        emit_progress(&app, job_id, &software_id, "uninstall", "uninstalling", "正在通过 Homebrew 卸载 Git formula…");
        let output = match homebrew::uninstall(&brew, &software_id) {
            Ok(value) => value,
            Err(error) => {
                let snapshot = detect_now(&app, &state);
                let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
                emit_finished(&app, job_id, &software_id, "uninstall", false, &format!("Homebrew 卸载失败：{error}"), &error, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: Some(system) }));
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, "uninstall", "verifying", "卸载完成，正在确认 Homebrew Git 已移除且系统 Git 仍可用…");
        let snapshot = detect_now(&app, &state);
        let homebrew_git = snapshot.instances.iter().any(|item| item.software_id == "git" && (item.source == "homebrewFormula" || item.path.starts_with(&brew.prefix)));
        let system_git = snapshot.instances.iter().find(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
        if homebrew_git {
            emit_finished(&app, job_id, &software_id, "uninstall", false, "Homebrew 命令结束，但仍检测到 Homebrew Git；未记录卸载成功", &output, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: Some(system_git.is_some()) }));
            return;
        }
        if let Some(record) = ownership {
            if let Ok(connection) = state.db.lock() {
                let _ = storage::mark_uninstalled(&connection, record.id, now_ms());
            }
        }
        let success = system_git.is_some();
        let message = if success { "Homebrew Git 已卸载，系统 Git 仍然可用" } else { "Homebrew Git 已卸载，但未检测到系统 Git；请检查环境" };
        emit_finished(&app, job_id, &software_id, "uninstall", success, message, &output, Some(Verification { verified: true, instance_path: None, version: None, health: None, system_git_available: Some(system_git.is_some()) }));
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
            refresh_detection,
            preview_homebrew_install,
            preview_homebrew_uninstall,
            start_homebrew_install,
            start_homebrew_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevHub V2");
}
