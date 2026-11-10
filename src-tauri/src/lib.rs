mod detection;
mod process;
mod storage;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use detection::DetectionSnapshot;
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

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    snapshot: Arc<Mutex<Option<DetectionSnapshot>>>,
    refreshing: Arc<AtomicBool>,
}

impl AppState {
    fn new(dir: std::path::PathBuf) -> Result<Self, String> {
        let connection = storage::open(&dir)?;
        let snapshot = storage::load(&connection);
        Ok(Self {
            db: Arc::new(Mutex::new(connection)),
            snapshot: Arc::new(Mutex::new(snapshot)),
            refreshing: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    AppStatus { phase: 2, product: "DevHub V2", detection: "read-only" }
}

#[tauri::command]
fn get_detection_snapshot(state: State<AppState>) -> Option<DetectionSnapshot> {
    state.snapshot.lock().ok().and_then(|value| value.clone())
}

/// Starts a background, read-only detection pass. Returns false when one is already running.
#[tauri::command]
fn refresh_detection(app: AppHandle, state: State<AppState>) -> bool {
    if state.refreshing.swap(true, Ordering::SeqCst) {
        return false;
    }
    let state = state.inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(detection::scan);
        let failed = result.is_err();
        let mut snapshot = match result {
            Ok(value) => value,
            Err(_) => DetectionSnapshot::failed("检测线程异常".into()),
        };
        if !failed {
            snapshot.stale = false;
        }
        if let Ok(connection) = state.db.lock() {
            if let Err(error) = storage::save(&connection, &snapshot) {
                snapshot.errors.push(detection::DetectionError { source: "cache".into(), message: error });
            }
        }
        if let Ok(mut cached) = state.snapshot.lock() {
            *cached = Some(snapshot.clone());
        }
        let _ = app.emit("detection://updated", &snapshot);
        state.refreshing.store(false, Ordering::SeqCst);
    });
    true
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
        .invoke_handler(tauri::generate_handler![get_app_status, get_detection_snapshot, refresh_detection])
        .run(tauri::generate_context!())
        .expect("error while running DevHub V2");
}
