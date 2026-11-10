use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    phase: u8,
    product: &'static str,
    detection: &'static str,
}

/// Phase 1 deliberately exposes no install, uninstall or detection command.
#[tauri::command]
fn get_app_status() -> AppStatus {
    AppStatus {
        phase: 1,
        product: "DevHub V2",
        detection: "not-wired",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_status])
        .run(tauri::generate_context!())
        .expect("error while running DevHub V2");
}
