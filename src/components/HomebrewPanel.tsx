import type { InstallFinished, InstallPreview, InstallProgress, UninstallPreview } from "../lib/ipc";

export type ConfirmAction = "install" | "uninstall" | null;

function verificationText(result: InstallFinished) {
  if (result.action === "uninstall") {
    return result.verification?.systemGitAvailable === false
      ? "系统 Git 未检测到，请检查环境"
      : "其他来源的 Git 未被修改";
  }
  const version = result.verification?.version;
  return version ? `已验证版本 ${version}` : "尚未验证到可用版本";
}

export function HomebrewPanel({ softwareId, preview, previewLoading, previewError, uninstallPreview, uninstallLoading, uninstallError, task, result, actionError, confirm, onConfirm, onInstall, onUninstall }: {
  softwareId: string;
  preview: InstallPreview | null;
  previewLoading: boolean;
  previewError: string | null;
  uninstallPreview: UninstallPreview | null;
  uninstallLoading: boolean;
  uninstallError: string | null;
  task: InstallProgress | null;
  result: InstallFinished | null;
  actionError: string | null;
  confirm: ConfirmAction;
  onConfirm: (action: ConfirmAction) => void;
  onInstall: () => void;
  onUninstall: () => void;
}) {
  const busy = task !== null;
  const canInstall = Boolean(preview?.canInstall);
  const canUninstall = Boolean(uninstallPreview?.canUninstall);
  const installTarget = preview?.package ? `${preview.package}${preview.formulaVersion ? ` ${preview.formulaVersion}` : ""}` : "Git";

  return (
    <section className="detail-block homebrew-panel">
      <h3>Homebrew 安装</h3>
      {previewLoading && <p className="detail-muted">正在检查 Homebrew 安装计划…</p>}
      {previewError && <p className="homebrew-error">{previewError}</p>}
      {preview && (
        <>
          {preview.canInstall ? (
            <>
              <p>{installTarget} · {preview.channel}</p>
              {preview.dependencies.length > 0 ? (
                <p className="detail-muted">依赖：{preview.dependencies.join("、")}</p>
              ) : <p className="detail-muted">没有额外依赖。</p>}
              {preview.newDependencies.length > 0 && <p className="detail-muted">将新增：{preview.newDependencies.join("、")}</p>}
              <p className="detail-muted">调用 `brew install --formula {preview.package}`；不自动升级其他包。</p>
            </>
          ) : preview.alreadyInstalled ? (
            <>
              <p>Homebrew 已安装 {preview.formulaVersion ?? "该包"}</p>
              {preview.ownership
                ? <p className="detail-muted">该包由 DevHub 本次安装，可安全卸载。</p>
                : <p className="detail-muted">该包是外部 Homebrew 安装，DevHub 仅查看，不自动卸载。</p>}
            </>
          ) : (
            <p className="homebrew-error">{preview.reason ?? "当前不能通过 Homebrew 安装"}</p>
          )}
        </>
      )}
      {!previewLoading && !preview && !previewError && softwareId !== "git" && (
        <p className="detail-muted">第三阶段先支持 Git 的 Homebrew 安装。</p>
      )}

      {uninstallLoading && <p className="detail-muted">正在检查卸载归属与依赖…</p>}
      {uninstallError && <p className="homebrew-error">{uninstallError}</p>}
      {uninstallPreview && !canUninstall && uninstallPreview.ownership && uninstallPreview.reason && (
        <p className="homebrew-error">不能卸载：{uninstallPreview.reason}</p>
      )}
      {uninstallPreview && canUninstall && uninstallPreview.dependents.length > 0 && (
        <p className="homebrew-error">依赖此包：{uninstallPreview.dependents.join("、")}</p>
      )}

      {confirm === "install" && (
        <div className="confirm-box">
          <strong>确认安装 Homebrew Git？</strong>
          <p>目标：{installTarget}。将调用 Homebrew 安装，可能拉取依赖；不会自动升级其他包。</p>
          {preview?.newDependencies.length ? <p>新增依赖：{preview.newDependencies.join("、")}</p> : null}
          <div className="confirm-actions">
            <button type="button" className="primary" onClick={onInstall}>确认安装</button>
            <button type="button" onClick={() => onConfirm(null)}>取消</button>
          </div>
        </div>
      )}
      {confirm === "uninstall" && (
        <div className="confirm-box">
          <strong>确认卸载 Homebrew Git？</strong>
          <p>只卸载 DevHub 记录的 Homebrew Git；不会执行 --zap 或 cleanup，不会删除系统 Git 或用户数据。</p>
          {uninstallPreview?.warnings.map(warning => <p key={warning}>{warning}</p>)}
          {uninstallPreview?.dependents.length ? <p className="homebrew-error">依赖：{uninstallPreview.dependents.join("、")}</p> : null}
          <div className="confirm-actions">
            <button type="button" className="primary" onClick={onUninstall}>确认卸载</button>
            <button type="button" onClick={() => onConfirm(null)}>取消</button>
          </div>
        </div>
      )}

      {task && (
        <div className="install-task" role="status">
          <div className="install-task-head"><span className="mini-spinner" />{task.message}</div>
          <div className={`indeterminate${task.percent !== null ? " determinate" : ""}`}>
            <span style={task.percent !== null ? { width: `${task.percent}%` } : undefined} />
          </div>
          {task.percent === null && <p className="detail-muted">Homebrew 未提供可靠百分比，使用不确定进度。</p>}
        </div>
      )}
      {actionError && <p className="homebrew-error">{actionError}</p>}
      {result && (
        <div className={`install-result ${result.success ? "success" : "failure"}`} role="status">
          <strong>{result.success ? "操作完成" : "操作未完成"}</strong>
          <p>{result.message}</p>
          <p className="detail-muted">{verificationText(result)}</p>
          {result.output && (
            <details className="install-output">
              <summary>Homebrew 输出</summary>
              <pre>{result.output}</pre>
            </details>
          )}
        </div>
      )}

      {confirm === null && (
        <div className="homebrew-actions">
          <button type="button" className="primary" aria-label="安装" disabled={busy || !canInstall} onClick={() => onConfirm("install")}>安装 Homebrew Git</button>
          <button type="button" aria-label="卸载" disabled={busy || !canUninstall} onClick={() => onConfirm("uninstall")}>卸载 Homebrew Git</button>
        </div>
      )}
    </section>
  );
}
