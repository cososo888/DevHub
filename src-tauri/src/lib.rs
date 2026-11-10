mod detection;
mod homebrew;
mod jdk;
mod process;
mod storage;
mod versions;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use detection::DetectionSnapshot;
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use versions::VersionCatalog;

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

fn version_catalog(state: &AppState, software_id: &str) -> VersionCatalog {
    let Some(snapshot) = snapshot_clone(state) else {
        return VersionCatalog { software_id: software_id.into(), versions: Vec::new(), error: Some("尚未完成本机检测，请先刷新".into()), homebrew_available: false, stale: true };
    };
    versions::catalog(software_id, &snapshot)
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
fn get_software_versions(software_id: String, state: State<AppState>) -> VersionCatalog {
    let live = version_catalog(&state, &software_id);
    if live.error.is_none() {
        if let (Ok(connection), Ok(payload)) = (state.db.lock(), serde_json::to_string(&live)) {
            let _ = storage::save_version_cache(&connection, &software_id, &payload, now_ms());
        }
    }
    let (cached, history) = match state.db.lock() {
        Ok(connection) => (storage::load_version_cache(&connection, &software_id).map(|(catalog, _)| catalog), storage::uninstall_history(&connection, &software_id)),
        Err(_) => (None, Vec::new()),
    };
    versions::merge_history(live, cached.as_ref(), &history)
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
fn start_homebrew_install(app: AppHandle, state: State<AppState>, software_id: String, version: String) -> Result<u64, String> {
    if !homebrew::supported(&software_id) {
        return Err("当前阶段暂时只支持部分软件的安装".into());
    }
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let formula = match homebrew::resolve_install(&software_id, &version) {
        Ok(value) => value,
        Err(error) => { state.install_busy.store(false, Ordering::SeqCst); return Err(error); }
    };
    let installed_same = snapshot_clone(&state).map(|snapshot| snapshot.instances.iter().any(|item| item.software_id == software_id && item.version.as_deref() == Some(version.as_str()))).unwrap_or(false);
    if installed_same {
        state.install_busy.store(false, Ordering::SeqCst);
        return Err("该版本已经存在于本机".into());
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        emit_progress(&app, job_id, &software_id, &version, "install", "installing", "正在通过成熟安装工具安装…");
        let started = now_ms();
        let ownership_id = match state.db.lock() {
            Ok(connection) => match storage::insert_pending(&connection, &software_id, &formula.name, "formula", false, started) {
                Ok(value) => Some(value),
                Err(error) => { emit_finished(&app, job_id, &software_id, &version, "install", false, &error, "", None); return; }
            },
            Err(_) => { emit_finished(&app, job_id, &software_id, &version, "install", false, "无法访问安装归属数据库", "", None); return; }
        };
        let output = match homebrew::install(&formula.name) {
            Ok(value) => value,
            Err(error) => {
                if let (Some(id), Ok(connection)) = (ownership_id, state.db.lock()) {
                    let _ = storage::update_install(&connection, id, "failed", None, Some(&version), Some(&error), now_ms());
                }
                let snapshot = detect_now(&app, &state);
                let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
                emit_finished(&app, job_id, &software_id, &version, "install", false, &format!("安装失败：{error}"), &error, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: if software_id == "git" { Some(system) } else { None } }));
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, &version, "install", "verifying", "安装命令结束，正在重新检测并验证版本…");
        let snapshot = detect_now(&app, &state);
        let cellar_marker = format!("/Cellar/{}/", formula.name);
        let opt_marker = format!("/opt/{}/", formula.name);
        let homebrew_instance = snapshot.instances.iter().find(|item| item.software_id == software_id && (item.path.contains(&cellar_marker) || item.path.contains(&opt_marker)) && item.health == "ok");
        match homebrew_instance {
            Some(instance) => {
                if let (Some(id), Ok(connection)) = (ownership_id, state.db.lock()) {
                    if let Err(error) = storage::update_install(&connection, id, "success", Some(&instance.path), instance.version.as_deref(), None, now_ms()) {
                        emit_finished(&app, job_id, &software_id, &version, "install", false, &error, &output, None);
                        return;
                    }
                }
                let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
                emit_finished(&app, job_id, &software_id, &version, "install", true, "安装完成并通过版本验证", &output, Some(Verification { verified: true, instance_path: Some(instance.path.clone()), version: instance.version.clone(), health: Some(instance.health.clone()), system_git_available: if software_id == "git" { Some(system) } else { None } }));
            }
            None => {
                if let (Some(id), Ok(connection)) = (ownership_id, state.db.lock()) {
                    let _ = storage::update_install(&connection, id, "verification_failed", None, Some(&version), Some("安装后未检测到可用实例"), now_ms());
                }
                emit_finished(&app, job_id, &software_id, &version, "install", false, "安装命令结束，但未验证到可用实例；不会授予卸载归属", &output, None);
            }
        }
    });
    Ok(job_id)
}

#[tauri::command]
fn start_homebrew_uninstall(app: AppHandle, state: State<AppState>, software_id: String, version: String, formula: Option<String>, instance_path: Option<String>) -> Result<u64, String> {
    let ownership = state.db.lock().ok().and_then(|connection| storage::active_for_version(&connection, &software_id, &version));
    if ownership.is_none() && formula.is_none() {
        let Some(instance_path) = instance_path else {
            return Err("没有找到该版本可安全卸载的安装方式".into());
        };
        let snapshot = snapshot_clone(&state).ok_or_else(|| "尚未完成本机检测".to_string())?;
        let exists = snapshot.instances.iter().any(|item| item.software_id == "jdk" && item.path == instance_path);
        if !exists {
            return Err("该 JDK 实例不在当前检测结果中".into());
        }
        let info = jdk::analyze(&instance_path);
        if !info.can_uninstall {
            return Err(info.reason.unwrap_or_else(|| "该 JDK 不能安全卸载".into()));
        }
        if state.install_busy.swap(true, Ordering::SeqCst) {
            return Err("已有安装或卸载任务正在执行".into());
        }
        let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let state = state.inner().clone();
        let app = app.clone();
        std::thread::spawn(move || {
            let _guard = BusyGuard(state.install_busy.clone());
            emit_progress(&app, job_id, &software_id, &version, "uninstall", "uninstalling", "正在将 JDK 安装包移入废纸篓…");
            let info = match jdk::uninstall(&instance_path) {
                Ok(value) => value,
                Err(error) => {
                    let snapshot = detect_now(&app, &state);
                    let others = snapshot.instances.iter().any(|item| item.software_id == "jdk" && item.path != instance_path);
                    emit_finished(&app, job_id, &software_id, &version, "uninstall", false, &format!("JDK 卸载失败：{error}"), &error, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: Some(others) }));
                    return;
                }
            };
            let snapshot = detect_now(&app, &state);
            let still_present = snapshot.instances.iter().any(|item| item.software_id == "jdk" && item.path == instance_path);
            let others = snapshot.instances.iter().any(|item| item.software_id == "jdk" && item.path != instance_path);
            if still_present {
                emit_finished(&app, job_id, &software_id, &version, "uninstall", false, "JDK 已移动，但检测仍然发现该实例；请检查", "", Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: Some(others) }));
                return;
            }
            let output = info.trash_path.unwrap_or_default();
            if let Ok(connection) = state.db.lock() {
                let _ = storage::record_uninstall(&connection, &software_id, &version, "jdkTrash", Some(&instance_path), now_ms());
            }
            emit_finished(&app, job_id, &software_id, &version, "uninstall", true, "JDK 已移入废纸篓，可从废纸篓恢复；其他 JDK 保留", &output, Some(Verification { verified: true, instance_path: None, version: None, health: None, system_git_available: Some(others) }));
        });
        return Ok(job_id);
    }
    let package = if let Some(record) = &ownership {
        record.package.clone()
    } else {
        let Some(formula) = formula else {
            return Err("没有找到该版本可安全卸载的安装方式".into());
        };
        if !homebrew::formulas_for(&software_id).contains(&formula.as_str()) {
            return Err("该安装方式不在允许的安全卸载范围内".into());
        }
        let snapshot = snapshot_clone(&state).ok_or_else(|| "尚未完成本机检测".to_string())?;
        let cellar = format!("/Cellar/{}/", formula);
        let opt = format!("/opt/{}/", formula);
        let target = snapshot.instances.iter().any(|item| item.software_id == software_id && item.version.as_deref() == Some(version.as_str()) && (item.path.contains(&cellar) || item.path.contains(&opt)));
        if !target {
            return Err("没有找到该版本对应的 Homebrew 安装实例".into());
        }
        formula
    };
    let info = homebrew::info(&package)?;
    if info.installed.is_empty() {
        return Err("该软件当前没有可卸载的安装".into());
    }
    if !homebrew::dependents(&package)?.is_empty() {
        return Err("该安装被其他软件依赖，不能安全卸载".into());
    }
    if state.install_busy.swap(true, Ordering::SeqCst) {
        return Err("已有安装或卸载任务正在执行".into());
    }
    let job_id = state.job_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = BusyGuard(state.install_busy.clone());
        emit_progress(&app, job_id, &software_id, &version, "uninstall", "uninstalling", "正在通过对应工具安全卸载…");
        let output = match homebrew::uninstall(&package) {
            Ok(value) => value,
            Err(error) => {
                let snapshot = detect_now(&app, &state);
                let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
                emit_finished(&app, job_id, &software_id, &version, "uninstall", false, &format!("卸载失败：{error}"), &error, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: if software_id == "git" { Some(system) } else { None } }));
                return;
            }
        };
        emit_progress(&app, job_id, &software_id, &version, "uninstall", "verifying", "卸载完成，正在重新检测其他来源…");
        let snapshot = detect_now(&app, &state);
        let cellar_marker = format!("/Cellar/{}/", package);
        let opt_marker = format!("/opt/{}/", package);
        let remaining = snapshot.instances.iter().any(|item| item.software_id == software_id && (item.path.contains(&cellar_marker) || item.path.contains(&opt_marker)));
        let system = snapshot.instances.iter().any(|item| item.software_id == "git" && item.path == "/usr/bin/git" && item.health == "ok");
        if remaining {
            emit_finished(&app, job_id, &software_id, &version, "uninstall", false, "卸载命令结束，但仍检测到 Homebrew 安装；未记录成功", &output, Some(Verification { verified: false, instance_path: None, version: None, health: None, system_git_available: if software_id == "git" { Some(system) } else { None } }));
            return;
        }
        if let Some(ownership) = ownership {
            if let Ok(connection) = state.db.lock() {
                let _ = storage::mark_uninstalled(&connection, ownership.id, now_ms());
                let _ = storage::record_uninstall(&connection, &software_id, &version, "homebrew", ownership.instance_path.as_deref(), now_ms());
            }
        } else if let Ok(connection) = state.db.lock() {
            let _ = storage::record_uninstall(&connection, &software_id, &version, "homebrew", None, now_ms());
        }
        let success = software_id != "git" || system;
        let message = if software_id == "git" {
            if system { "已卸载 Homebrew Git，系统 Git 仍然可用" } else { "已卸载 Homebrew Git，但未检测到系统 Git；请检查环境" }
        } else { "已安全卸载该版本，其他来源保持不变" };
        emit_finished(&app, job_id, &software_id, &version, "uninstall", success, message, &output, Some(Verification { verified: true, instance_path: None, version: None, health: None, system_git_available: if software_id == "git" { Some(system) } else { None } }));
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
            get_software_versions,
            refresh_detection,
            start_homebrew_install,
            start_homebrew_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevHub V2");
}
