//! macOS UserNotifications: real consent, foreground banners and observable delivery errors.
//! No arbitrary notification text, URLs or commands are accepted from the webview.
use std::sync::Mutex;

use serde::Serialize;
use tauri::{App, AppHandle, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationStatus {
    permission: &'static str,
    alerts_enabled: bool,
    banners_enabled: bool,
    last_error: Option<String>,
}

struct NotificationRuntime {
    bundled: bool,
    last_error: Mutex<Option<String>>,
}

pub(crate) fn setup(app: &mut App) {
    #[cfg(target_os = "macos")]
    let bundled = macos::setup(app.handle());
    #[cfg(not(target_os = "macos"))]
    let bundled = false;
    app.manage(NotificationRuntime {
        bundled,
        last_error: Mutex::new(None),
    });
}

fn record_error(app: &AppHandle, error: Option<String>) {
    *app.state::<NotificationRuntime>()
        .last_error
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = error;
}

fn status(app: &AppHandle) -> Result<NotificationStatus, String> {
    let runtime = app.state::<NotificationRuntime>();
    #[cfg(target_os = "macos")]
    let mut result = objc2::rc::autoreleasepool(|_| macos::status(runtime.bundled))?;
    #[cfg(not(target_os = "macos"))]
    let mut result = unavailable();
    result.last_error = runtime
        .last_error
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    Ok(result)
}

fn unavailable() -> NotificationStatus {
    NotificationStatus {
        permission: "unavailable",
        alerts_enabled: false,
        banners_enabled: false,
        last_error: None,
    }
}

#[tauri::command]
pub(crate) async fn get_notification_status(app: AppHandle) -> Result<NotificationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || status(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn request_notification_permission(
    app: AppHandle,
) -> Result<NotificationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let current = status(&app)?;
        if current.permission != "notDetermined" {
            return Ok(current);
        }
        #[cfg(target_os = "macos")]
        objc2::rc::autoreleasepool(|_| macos::request_permission())?;
        record_error(&app, None);
        status(&app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn send_test_notification(app: AppHandle) -> Result<NotificationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = send(
            &app,
            "DevHub 通知测试",
            "能看到这条提醒，说明系统通知已经可用。",
        );
        record_error(&app, result.as_ref().err().cloned());
        result?;
        status(&app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn open_notification_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Fixed system URL; only opens Settings. The user chooses whether to change anything.
        tauri::async_runtime::spawn_blocking(|| {
            let output = crate::process::run(
                std::path::Path::new("/usr/bin/open"),
                &["x-apple.systempreferences:com.apple.Notifications-Settings.extension"],
                std::time::Duration::from_secs(5),
                16 * 1024,
            )?;
            if output.status != 0 {
                return Err(format!("无法打开通知设置：{}", output.stderr.trim()));
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "macos"))]
    Err("当前只支持 macOS 通知".into())
}

fn send(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    let current = status(app)?;
    match current.permission {
        "unavailable" => return Err("请从构建后的 DevHub V2.app 使用系统通知".into()),
        "notDetermined" => return Err("尚未允许通知，请先点击“开启通知”".into()),
        "denied" => return Err("系统通知已关闭，请在通知设置中允许 DevHub 提醒".into()),
        _ => {}
    }
    if !current.alerts_enabled {
        return Err("通知提醒已关闭，请在通知设置中开启".into());
    }
    #[cfg(target_os = "macos")]
    return objc2::rc::autoreleasepool(|_| macos::send(title, body));
    #[cfg(not(target_os = "macos"))]
    Err("当前只支持 macOS 通知".into())
}

pub(crate) fn installation_complete(app: &AppHandle, package: &str) {
    let app = app.clone();
    let body = format!("{package} 已安装成功");
    // The finished install never waits for Notification Center and never fails because of it.
    tauri::async_runtime::spawn_blocking(move || {
        let result = send(&app, "安装完成", &body);
        if let Err(error) = &result {
            eprintln!("DevHub notification: {error}");
        }
        record_error(&app, result.err());
    });
}

#[cfg(target_os = "macos")]
mod macos {
    use super::NotificationStatus;
    use block2::{DynBlock, RcBlock};
    use objc2::{
        AnyThread, DefinedClass, define_class, msg_send,
        rc::Retained,
        runtime::{Bool, ProtocolObject},
    };
    use objc2_foundation::{NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
    use objc2_user_notifications::{
        UNAlertStyle, UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent,
        UNNotification, UNNotificationDefaultActionIdentifier, UNNotificationPresentationOptions,
        UNNotificationRequest, UNNotificationResponse, UNNotificationSetting,
        UNNotificationSettings, UNNotificationSound, UNUserNotificationCenter,
        UNUserNotificationCenterDelegate,
    };
    use std::{cell::RefCell, ptr::NonNull, sync::mpsc, time::Duration};
    use tauri::AppHandle;

    struct DelegateIvars {
        app: AppHandle,
    }

    define_class!(
        // SAFETY: NSObject permits subclassing. Only the thread-safe AppHandle is stored;
        // Notification Center may call these delegate methods from a background queue.
        #[unsafe(super = NSObject)]
        #[ivars = DelegateIvars]
        struct DevHubNotificationDelegate;

        unsafe impl NSObjectProtocol for DevHubNotificationDelegate {}
        unsafe impl UNUserNotificationCenterDelegate for DevHubNotificationDelegate {
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                completion.call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Sound,));
            }

            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                completion: &DynBlock<dyn Fn()>,
            ) {
                // SAFETY: Apple exports a process-lifetime NSString constant.
                if &*response.actionIdentifier() == unsafe { UNNotificationDefaultActionIdentifier }
                {
                    let app = self.ivars().app.clone();
                    let to_show = app.clone();
                    let _ = app.run_on_main_thread(move || crate::desktop::show_main(&to_show));
                }
                completion.call(());
            }
        }
    );

    thread_local! {
        // UNUserNotificationCenter.delegate is weak. Retain it for the main thread's lifetime.
        static DELEGATE: RefCell<Option<Retained<DevHubNotificationDelegate>>> = const { RefCell::new(None) };
    }

    pub(super) fn setup(app: &AppHandle) -> bool {
        let bundled = NSBundle::mainBundle()
            .bundleIdentifier()
            .is_some_and(|id| id.to_string() == app.config().identifier);
        // Calling currentNotificationCenter from an unbundled `tauri dev` binary can throw.
        if !bundled {
            return false;
        }
        let object =
            DevHubNotificationDelegate::alloc().set_ivars(DelegateIvars { app: app.clone() });
        // SAFETY: Fresh allocation with initialized Rust ivars and NSObject's designated init.
        let delegate: Retained<DevHubNotificationDelegate> =
            unsafe { msg_send![super(object), init] };
        UNUserNotificationCenter::currentNotificationCenter()
            .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        DELEGATE.with(|slot| *slot.borrow_mut() = Some(delegate));
        true
    }

    pub(super) fn status(bundled: bool) -> Result<NotificationStatus, String> {
        if !bundled {
            return Ok(super::unavailable());
        }
        let (tx, rx) = mpsc::channel();
        let callback = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
            // SAFETY: The framework supplies a valid settings object for the callback's duration.
            let settings = unsafe { settings.as_ref() };
            let permission = match settings.authorizationStatus() {
                UNAuthorizationStatus::NotDetermined => "notDetermined",
                UNAuthorizationStatus::Denied => "denied",
                UNAuthorizationStatus::Authorized
                | UNAuthorizationStatus::Provisional
                | UNAuthorizationStatus::Ephemeral => "granted",
                _ => "unavailable",
            };
            let _ = tx.send(NotificationStatus {
                permission,
                alerts_enabled: settings.alertSetting() == UNNotificationSetting::Enabled,
                banners_enabled: settings.alertStyle() != UNAlertStyle::None,
                last_error: None,
            });
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationSettingsWithCompletionHandler(&callback);
        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "读取系统通知设置超时，请稍后重试".into())
    }

    pub(super) fn request_permission() -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        let callback = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
            let _ = tx.send(error_result(error));
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
                &callback,
            );
        rx.recv_timeout(Duration::from_secs(90))
            .map_err(|_| "等待通知授权超时，可在通知设置中确认后重试".to_string())?
    }

    fn error_result(error: *mut NSError) -> Result<(), String> {
        // SAFETY: A non-null NSError is owned by the framework for the callback's duration.
        match unsafe { error.as_ref() } {
            Some(error) => Err(error.localizedDescription().to_string()),
            None => Ok(()),
        }
    }

    pub(super) fn send(title: &str, body: &str) -> Result<(), String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        content.setBody(&NSString::from_str(body));
        content.setSound(Some(&UNNotificationSound::defaultSound()));
        let identifier = NSString::from_str(&format!(
            "devhub-{}-{}",
            crate::now_ms(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &identifier,
            &content,
            None,
        );
        let (tx, rx) = mpsc::channel();
        let callback = RcBlock::new(move |error: *mut NSError| {
            let _ = tx.send(error_result(error));
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&callback));
        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "系统未确认通知提交，请检查通知设置".to_string())?
    }
}
