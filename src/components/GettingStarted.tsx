import { useState } from "react";
import { desktop } from "../lib/ipc";
import { NotificationSettings } from "./NotificationSettings";

export type BrewReadiness = "checking" | "missing" | "ready" | "error";

export function GettingStarted({ state, problem, onRefresh, onDismiss }: {
  state: BrewReadiness;
  problem: string | null;
  onRefresh: () => void;
  onDismiss: () => void;
}) {
  const [siteError, setSiteError] = useState(false);
  async function openSite() {
    setSiteError(false);
    try { await desktop("open_homebrew_site"); }
    catch { setSiteError(true); }
  }

  return <section className="getting-started" id="getting-started" tabIndex={-1} aria-labelledby="guide-heading">
    <header><h2 id="guide-heading">使用指南</h2><button type="button" onClick={onDismiss}>收起</button></header>
    <p>点软件图标查看用途和本机版本，点顶栏放大镜搜索软件。</p>
    <div className="guide-install">
      <h3>安装第一款软件</h3>
      {state === "ready" && <p role="status">Homebrew 已就绪。打开软件详情，点「安装」，核对软件名后点「确认」。</p>}
      {state === "checking" && <p role="status">正在检查本机的 Homebrew…</p>}
      {state === "error" && <>
        <p role="status">{problem || "暂时无法确认 Homebrew 状态。"} 你可以继续浏览软件。</p>
        <button type="button" onClick={onRefresh}>重新检测</button>
      </>}
      {state === "missing" && <>
        <p>在常见安装位置未找到 Homebrew。浏览和查看本机环境可直接使用，安装软件需要先准备 Homebrew。</p>
        <ol>
          <li>打开 Homebrew 官网，按 macOS 要求下载并运行官方 <code>.pkg</code> 安装包。</li>
          <li>安装完成后回到这里，点「重新检测」。</li>
        </ol>
        <div className="guide-actions">
          <button type="button" onClick={() => void openSite()}>打开 Homebrew 官网 ↗</button>
          <button type="button" onClick={onRefresh}>重新检测</button>
        </div>
        {siteError && <p role="alert">无法打开浏览器，请手动访问 https://brew.sh/。</p>}
      </>}
    </div>
    <NotificationSettings />
  </section>;
}
