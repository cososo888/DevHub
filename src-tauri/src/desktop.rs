//! Desktop lifetime and backend-owned completion notifications.
//! Closing the main window keeps its webview and in-flight work alive; quitting is untouched.
use tauri::{App, AppHandle, Manager, RunEvent, Window, WindowEvent};
use tauri_plugin_notification::NotificationExt;

pub(crate) fn show_main(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Err(error) = app.show() {
        eprintln!("DevHub could not show the application: {error}");
    }
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.unminimize().and_then(|_| window.show()).and_then(|_| window.set_focus()) {
            eprintln!("DevHub could not restore its window: {error}");
        }
    }
}

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular);

    let handle = app.handle().clone();
    app.notification().on_action(move |_| {
        let to_show = handle.clone();
        if let Err(error) = handle.run_on_main_thread(move || show_main(&to_show)) {
            eprintln!("DevHub could not handle a notification click: {error}");
        }
    })?;
    Ok(())
}

pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    #[cfg(target_os = "macos")]
    if window.label() == "main" {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(error) = window.hide() {
                eprintln!("DevHub could not hide its window: {error}");
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, event);
}

pub(crate) fn on_run_event(app: &AppHandle, event: &RunEvent) {
    #[cfg(target_os = "macos")]
    if let RunEvent::Reopen { .. } = event {
        show_main(app);
    }
    // Do not prevent ExitRequested: Dock Quit and Cmd+Q must really quit.
    #[cfg(not(target_os = "macos"))]
    let _ = (app, event);
}

/// Called once from the verified install-success path, never from frontend progress events.
pub(crate) fn installation_complete(app: &AppHandle, package: &str) {
    if let Err(error) = app.notification().builder()
        .title("安装完成")
        .body(format!("{package} 已安装成功"))
        .show()
    {
        // Notification delivery is best effort and must not turn a successful install into failure.
        eprintln!("DevHub could not submit the completion notification: {error}");
    }
}
