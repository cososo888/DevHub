import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Software } from "../catalog";
import { useReducedMotion } from "../hooks/useReducedMotion";
import type { InstallFinished, InstallProgress, PackagePlan, SoftwareInstance } from "../lib/ipc";
import { SoftwareLogo } from "./SoftwareLogo";

export interface OrbOrigin {
  left: number;
  top: number;
  width: number;
  height: number;
}

type Confirm = "install" | "uninstall" | null;

const CARD_WIDTH = 380;
const CARD_MIN_HEIGHT = 140;
const PAGE_PADDING = 16;
const RADIUS = 24;
const CLOSE_MS = 320;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

/** The orb is one corner of the expanded card; never center a detached dialog. */
export function anchoredCard(origin: OrbOrigin, width: number, height: number, viewportWidth = window.innerWidth, viewportHeight = window.innerHeight) {
  const w = Math.min(width, viewportWidth - PAGE_PADDING * 2);
  const maxHeight = viewportHeight - PAGE_PADDING * 2;
  const centerX = origin.left + origin.width / 2;
  const left = centerX < viewportWidth / 2
    ? clamp(origin.left, PAGE_PADDING, viewportWidth - w - PAGE_PADDING)
    : clamp(origin.left + origin.width - w, PAGE_PADDING, viewportWidth - w - PAGE_PADDING);
  const below = Math.max(0, viewportHeight - (origin.top + origin.height) - PAGE_PADDING);
  const above = Math.max(0, origin.top - PAGE_PADDING);
  const natural = Math.max(0, Math.min(height, maxHeight));
  let top: number;
  if (natural <= below) top = origin.top;
  else if (natural <= above) top = origin.top + origin.height - natural;
  else top = clamp(origin.top, PAGE_PADDING, viewportHeight - natural - PAGE_PADDING);
  top = clamp(top, PAGE_PADDING, viewportHeight - natural - PAGE_PADDING);
  return { left, top, width: w, height: natural };
}

function isBrewManaged(instance: SoftwareInstance) {
  return instance.source === "homebrewFormula"
    || instance.source === "homebrewCask"
    || instance.path.includes("/Cellar/")
    || instance.path.includes("/Caskroom/")
    || instance.path.includes("/opt/homebrew/");
}

function DetailBody({ item, instances, plan, homebrewReady, task, result, actionError, confirm, onConfirm, onInstall, onUninstall, interactive = false }: {
  item: Software;
  instances: SoftwareInstance[];
  plan: PackagePlan | null;
  homebrewReady: boolean;
  task: InstallProgress | null;
  result: InstallFinished | null;
  actionError: string | null;
  confirm: Confirm;
  onConfirm: (value: Confirm) => void;
  onInstall: () => void;
  onUninstall: () => void;
  interactive?: boolean;
}) {
  const installed = instances.length > 0;
  const canUninstall = instances.some(isBrewManaged);
  const busy = Boolean(task);
  const log = result?.output || actionError || "";
  return (
    <div className="detail-inner" tabIndex={interactive ? 0 : undefined} role={interactive ? "region" : undefined} aria-label={interactive ? `${item.name} 详情内容` : undefined}>
      <header className="detail-head">
        <h2>{item.name}</h2>
        <p>{item.description}</p>
      </header>

      <div className="env-summary">
        <span className={`env-state${installed ? " installed" : ""}`}>{installed ? "已安装" : "未安装"}</span>
        {installed && !canUninstall && <span className="managed-note">系统或其他方式安装，仅查看</span>}
      </div>

      {instances.length > 0 && (
        <div className="instance-list">
          {instances.map(instance => (
            <div className="instance-line" key={`${instance.path}:${instance.version ?? ""}`}>
              <span>{instance.version ?? "版本未知"}</span>
              <small>{instance.path}</small>
            </div>
          ))}
        </div>
      )}

      <div className="env-actions">
        {!installed && plan && (
          <button type="button" className="primary" aria-label={`安装 ${item.name}`} disabled={!homebrewReady || busy || Boolean(confirm)} onClick={() => onConfirm("install")}>安装</button>
        )}
        {installed && canUninstall && plan && (
          <button type="button" aria-label={`卸载 ${item.name}`} disabled={busy || Boolean(confirm)} onClick={() => onConfirm("uninstall")}>卸载</button>
        )}
        {!plan && <span className="managed-note">暂无可靠的终端安装方式</span>}
        {plan && !homebrewReady && !installed && <span className="managed-note">安装需要 Homebrew</span>}
      </div>

      {confirm && plan && (
        <div className="row-confirm">
          <span>{confirm === "install" ? `安装 ${item.name}？` : `卸载 ${item.name}？`}</span>
          <small className="command-line">{confirm === "install" ? plan.installCommand : plan.uninstallCommand}</small>
          {confirm === "uninstall" && <small>只移除 Homebrew 管理的文件；不会删除项目或用户数据。</small>}
          <div className="row-confirm-actions">
            <button type="button" className="primary" onClick={() => { onConfirm(null); confirm === "install" ? onInstall() : onUninstall(); }}>确认</button>
            <button type="button" onClick={() => onConfirm(null)}>取消</button>
          </div>
        </div>
      )}

      {task && (
        <div className="row-progress" role="status">
          <span className="mini-spinner" />
          <span>{task.message}</span>
          <div className="indeterminate" />
        </div>
      )}
      {result && (
        <div className={result.success ? "row-result" : "row-error"}>
          <span>{result.message}</span>
          {log && (
            <details className="task-log">
              <summary>技术日志</summary>
              <pre>{log}</pre>
            </details>
          )}
        </div>
      )}
      {!result && actionError && (
        <div className="row-error">
          <span>{actionError}</span>
          <details className="task-log"><summary>技术日志</summary><pre>{actionError}</pre></details>
        </div>
      )}
    </div>
  );
}

export function DetailOverlay({ item, origin, onClose, instances = [], plan = null, homebrewReady = false, task = null, result = null, actionError = null, onInstall, onUninstall }: {
  item: Software;
  origin: OrbOrigin;
  onClose: () => void;
  instances?: SoftwareInstance[];
  plan?: PackagePlan | null;
  homebrewReady?: boolean;
  task?: InstallProgress | null;
  result?: InstallFinished | null;
  actionError?: string | null;
  onInstall: () => void;
  onUninstall: () => void;
}) {
  const reduced = useReducedMotion();
  const [viewport, setViewport] = useState({ width: window.innerWidth, height: window.innerHeight });
  const [target, setTarget] = useState(() => anchoredCard(origin, CARD_WIDTH, CARD_MIN_HEIGHT));
  const [entered, setEntered] = useState(false);
  const [closing, setClosing] = useState(false);
  const [confirm, setConfirm] = useState<Confirm>(null);
  const measure = useRef<HTMLDivElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const card = useRef<HTMLElement>(null);
  const timer = useRef<number | null>(null);
  const width = Math.min(CARD_WIDTH, viewport.width - PAGE_PADDING * 2);
  const instanceSignature = instances.map(instance => `${instance.path}:${instance.version ?? ""}`).join("|");

  useEffect(() => {
    const onResize = () => setViewport({ width: window.innerWidth, height: window.innerHeight });
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const close = useCallback(() => {
    if (timer.current !== null) return;
    if (reduced) { onClose(); return; }
    setClosing(true);
    setEntered(false);
    timer.current = window.setTimeout(onClose, CLOSE_MS);
  }, [onClose, reduced]);

  useLayoutEffect(() => {
    const natural = Math.max(measure.current?.scrollHeight ?? 0, CARD_MIN_HEIGHT);
    setTarget(anchoredCard(origin, width, natural, viewport.width, viewport.height));
    if (reduced) { setEntered(true); return; }
    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => setEntered(true));
    });
    return () => { cancelAnimationFrame(first); if (second) cancelAnimationFrame(second); };
  }, [origin.left, origin.top, origin.width, origin.height, width, viewport.width, viewport.height, reduced, instanceSignature, task?.message, result, actionError, confirm, plan?.package, homebrewReady]);

  useEffect(() => {
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    closeButton.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); close(); return; }
      const target = event.target;
      if (target instanceof HTMLElement && target.closest("input, select, textarea")) return;
      const content = card.current?.querySelector<HTMLElement>(".detail-inner");
      if (!content || content.scrollHeight <= content.clientHeight) return;
      const page = Math.max(40, content.clientHeight * 0.85);
      if (event.key === "PageDown" || event.key === "ArrowDown") { content.scrollBy({ top: event.key === "PageDown" ? page : 40 }); event.preventDefault(); }
      else if (event.key === "PageUp" || event.key === "ArrowUp") { content.scrollBy({ top: event.key === "PageUp" ? -page : -40 }); event.preventDefault(); }
      else if (event.key === "Home") { content.scrollTo({ top: 0 }); event.preventDefault(); }
      else if (event.key === "End") { content.scrollTo({ top: content.scrollHeight }); event.preventDefault(); }
    };
    window.addEventListener("keydown", onKey);
    return () => { document.body.style.overflow = previous; window.removeEventListener("keydown", onKey); };
  }, [close]);

  useEffect(() => () => { if (timer.current !== null) window.clearTimeout(timer.current); }, []);

  const geometry: CSSProperties = entered
    ? { left: target.left, top: target.top, width: target.width, height: target.height, borderRadius: RADIUS }
    : { left: origin.left, top: origin.top, width: origin.width, height: origin.height, borderRadius: Math.max(1, origin.width / 2) };
  const body = <DetailBody item={item} instances={instances} plan={plan} homebrewReady={homebrewReady} task={task} result={result} actionError={actionError} confirm={confirm} onConfirm={setConfirm} onInstall={onInstall} onUninstall={onUninstall} interactive />;

  return createPortal(
    <div className="detail-layer">
      <div className="detail-catcher" onClick={close} aria-hidden="true" />
      <div ref={measure} className="detail-measure" style={{ width }} aria-hidden="true">
        <DetailBody item={item} instances={instances} plan={plan} homebrewReady={homebrewReady} task={task} result={result} actionError={actionError} confirm={confirm} onConfirm={setConfirm} onInstall={onInstall} onUninstall={onUninstall} />
      </div>
      <section ref={card} className={`morph-card${entered ? " open" : ""}${closing ? " closing" : ""}`} style={{ ...geometry, "--orb-accent": item.accent } as CSSProperties} role="dialog" aria-modal="true" aria-label={`${item.name} 详情`} onClick={event => event.stopPropagation()}>
        <span className="morph-logo" aria-hidden="true"><SoftwareLogo id={item.id} name={item.name} /></span>
        <button ref={closeButton} type="button" className="detail-close" aria-label="关闭详情" onClick={close}>×</button>
        {body}
      </section>
    </div>,
    document.querySelector(".app") ?? document.body
  );
}
