import { useState } from "react";
import { SOFTWARE } from "../catalog";
import type { InstallFinished } from "../lib/ipc";

/** An in-app receipt remains visible even when macOS suppresses a system banner. */
export function InstallationNotice({ result }: { result: InstallFinished | null }) {
  const [dismissed, setDismissed] = useState<number | null>(null);
  if (!result || !result.success || !result.verification?.verified || result.action !== "install" || result.jobId === dismissed) return null;
  const name = SOFTWARE.find(item => item.package === result.softwareId && item.kind === result.kind)?.name ?? result.softwareId;
  return <div className="completion-notice" role="status">
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m8 12 3 3 5-6"/></svg>
    <span><strong>{name}</strong>安装完成</span>
    <button type="button" className="icon-button" aria-label="关闭安装完成提醒" onClick={() => setDismissed(result.jobId)}>×</button>
  </div>;
}
