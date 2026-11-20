import { useCallback, useEffect, useState } from "react";
import { desktop } from "../lib/ipc";
import type { NotificationStatus } from "../lib/ipc";

function statusText(status: NotificationStatus | null) {
  if (!status) return "正在读取通知设置…";
  if (status.permission === "unavailable") return "请从 DevHub V2.app 使用系统通知";
  if (status.permission === "notDetermined") return "尚未开启安装提醒";
  if (status.permission === "denied") return "系统通知已关闭";
  if (!status.alertsEnabled) return "系统通知提醒未开启";
  return status.bannersEnabled ? "安装完成时发送系统提醒" : "通知已允许，横幅未开启";
}

export function NotificationSettings() {
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<NotificationStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const refresh = useCallback(() => {
    void desktop<NotificationStatus>("get_notification_status").then(setStatus).catch(error => setFeedback(String(error)));
  }, []);

  useEffect(() => {
    if (!open) return;
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [open, refresh]);

  async function perform(command: "request_notification_permission" | "send_test_notification") {
    setBusy(true); setFeedback(null);
    try {
      const next = await desktop<NotificationStatus>(command);
      setStatus(next);
      if (command === "send_test_notification") setFeedback("已提交通知。如果未看到提醒，请检查系统横幅和专注模式。");
    } catch (error) { setFeedback(String(error)); refresh(); }
    finally { setBusy(false); }
  }

  return <details className="guide-notifications" onToggle={event => setOpen(event.currentTarget.open)}>
    <summary>安装提醒设置</summary>
    {open && <div className="notification-settings">
      <p role="status">{!status && feedback ? "无法读取通知设置" : statusText(status)}</p>
      {!status && feedback && <button type="button" onClick={() => { setFeedback(null); refresh(); }}>重新读取</button>}
      {status?.permission === "notDetermined" && <button type="button" disabled={busy} onClick={() => void perform("request_notification_permission")}>{busy ? "等待系统确认…" : "开启通知"}</button>}
      {status?.permission === "granted" && <button type="button" disabled={busy || !status.alertsEnabled} onClick={() => void perform("send_test_notification")}>{busy ? "正在发送…" : "发送测试通知"}</button>}
      {status?.permission !== "unavailable" && <button type="button" disabled={busy} onClick={() => void desktop("open_notification_settings").catch(error => setFeedback(String(error)))}>打开系统通知设置</button>}
      {(feedback || status?.lastError) && <p className="notification-feedback" role="status">{feedback || status?.lastError}</p>}
    </div>}
  </details>;
}
