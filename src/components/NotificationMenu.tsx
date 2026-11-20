import { useCallback, useEffect, useId, useRef, useState } from "react";
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

export function NotificationMenu() {
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<NotificationStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const id = useId();
  const refresh = useCallback(() => {
    void desktop<NotificationStatus>("get_notification_status").then(setStatus).catch(error => setFeedback(String(error)));
  }, []);

  useEffect(() => {
    if (!open) return;
    panel.current?.focus();
    refresh();
    const dismiss = (event: Event) => { if (event.target instanceof Node && !root.current?.contains(event.target)) setOpen(false); };
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("focusin", dismiss);
    window.addEventListener("focus", refresh);
    return () => { document.removeEventListener("pointerdown", dismiss); document.removeEventListener("focusin", dismiss); window.removeEventListener("focus", refresh); };
  }, [open, refresh]);

  async function perform(command: "request_notification_permission" | "send_test_notification") {
    setBusy(true); setFeedback(null);
    try {
      const next = await desktop<NotificationStatus>(command);
      setStatus(next);
      if (command === "send_test_notification") setFeedback("已提交到系统；若未弹出，请检查横幅和专注模式。");
    } catch (error) { setFeedback(String(error)); refresh(); }
    finally { setBusy(false); }
  }

  return <div className="notification-picker" ref={root} onKeyDown={event => {
    if (open && event.key === "Escape") { event.preventDefault(); event.stopPropagation(); setOpen(false); trigger.current?.focus(); }
  }}>
    <button ref={trigger} type="button" className="icon-button" aria-label="通知设置" title="安装提醒" aria-haspopup="dialog" aria-expanded={open} aria-controls={open ? id : undefined} onClick={() => { setFeedback(null); setOpen(value => !value); }}>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4"/></svg>
    </button>
    {open && <div className="notification-menu" ref={panel} tabIndex={-1} id={id} role="dialog" aria-label="安装通知设置">
      <strong>安装提醒</strong><p role="status">{!status && feedback ? "无法读取通知设置" : statusText(status)}</p>
      {!status && feedback && <button type="button" onClick={() => { setFeedback(null); refresh(); }}>重新读取</button>}
      {status?.permission === "notDetermined" && <button type="button" disabled={busy} onClick={() => void perform("request_notification_permission")}>{busy ? "等待系统确认…" : "开启通知"}</button>}
      {status?.permission === "granted" && <button type="button" disabled={busy || !status.alertsEnabled} onClick={() => void perform("send_test_notification")}>{busy ? "正在发送…" : "发送测试通知"}</button>}
      {status?.permission !== "unavailable" && <button type="button" disabled={busy} onClick={() => void desktop("open_notification_settings").catch(error => setFeedback(String(error)))}>打开系统通知设置</button>}
      {(feedback || status?.lastError) && <p className="notification-feedback" role="status">{feedback || status?.lastError}</p>}
    </div>}
  </div>;
}
