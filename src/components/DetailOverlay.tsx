import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Software } from "../catalog";
import { useReducedMotion } from "../hooks/useReducedMotion";
import type { InstallFinished, InstallProgress, SoftwareInstance } from "../lib/ipc";
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

function DetailBody({ item, instances, brewInstalled, brewVersion, brewPath, homebrewReady, task, result, actionError, confirm, onConfirm, onInstall, onUninstall, interactive = false }: {
  item: Software;
  instances: SoftwareInstance[];
  brewInstalled: boolean;
  brewVersion: string | null;
  brewPath: string | null;
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
  const installed = instances.length > 0 || brewInstalled;
  const nonBrewInstalled = instances.length > 0 && !brewInstalled;
  const busy = Boolean(task);
  const packageName = item.package;
  const installCommand = item.kind === "cask" ? `brew install --cask ${packageName}` : `brew install ${packageName}`;
  const uninstallCommand = item.kind === "cask" ? `brew uninstall --cask ${packageName}` : `brew uninstall ${packageName}`;
  const showBrewLine = brewInstalled && !instances.some(instance => instance.path.includes("/Cellar/") || instance.path.includes("/Caskroom/") || instance.path.includes("/opt/homebrew/"));
  const log = result?.output || actionError || "";
  return (
    <div className="detail-inner" tabIndex={interactive ? 0 : undefined} role={interactive ? "region" : undefined} aria-label={interactive ? `${item.name} 详情内容` : undefined}>
      <header className="detail-head">
        <h2>{item.name}</h2>
        <p>{item.description}</p>
      </header>

      <div className="env-summary">
        <span className={`env-state${installed ? " installed" : ""}`}>{installed ? "已安装" : "未安装"}</span>
        {brewInstalled && <span className="managed-note">Homebrew 管理</span>}
        {nonBrewInstalled && <span className="managed-note">系统或其他方式安装，仅查看</span>}
      </div>

      {(instances.length > 0 || showBrewLine) && (
        <div className="instance-list">
          {instances.map(instance => (
            <div className="instance-line" key={`${instance.path}:${instance.version ?? ""}`}>
              <span>{instance.version ?? "版本未知"}</span>
              <small>{instance.path}</small>
            </div>
          ))}
          {showBrewLine && (
            <div className="instance-line" key="brew-package">
              <span>{brewVersion ?? "已安装"}</span>
              <small>{brewPath ?? `Homebrew · ${packageName}`}</small>
            </div>
          )}
        </div>
      )}

      <div className="env-actions">
        {!brewInstalled && !busy && !confirm && (
          <button type="button" className="primary" aria-label={`安装 ${item.name}`} disabled={!homebrewReady} onClick={() => onConfirm("install")}>安装</button>
        )}
        {brewInstalled && !busy && !confirm && (
          <button type="button" aria-label={`卸载 ${item.name}`} onClick={() => onConfirm("uninstall")}>卸载</button>
        )}
        {!homebrewReady && !brewInstalled && <span className="managed-note">安装需要 Homebrew</span>}
      </div>

      {confirm && (
        <div className="row-confirm">
          <span>{confirm === "install" ? `安装 ${item.name}？` : `卸载 ${item.name}？`}</span>
          <small className="command-line">{confirm === "install" ? installCommand : uninstallCommand}</small>
          {confirm === "install" && nonBrewInstalled && <small>本机已有其他来源的安装；继续会再安装一份 Homebrew 版本。</small>}
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

export function DetailOverlay({ item, origin, onClose, instances = [], brewInstalled = false, brewVersion = null, brewPath = null, homebrewReady = false, task = null, result = null, actionError = null, onInstall, onUninstall }: {
  item: Software;
  origin: OrbOrigin;
  onClose: () => void;
  instances?: SoftwareInstance[];
  brewInstalled?: boolean;
  brewVersion?: string | null;
  brewPath?: string | null;
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
  }, [origin.left, origin.top, origin.width, origin.height, width, viewport.width, viewport.height, reduced, instanceSignature, task?.message, result, actionError, confirm, item.package, brewInstalled, brewVersion, brewPath, homebrewReady]);

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
  const body = <DetailBody item={item} instances={instances} brewInstalled={brewInstalled} brewVersion={brewVersion} brewPath={brewPath} homebrewReady={homebrewReady} task={task} result={result} actionError={actionError} confirm={confirm} onConfirm={setConfirm} onInstall={onInstall} onUninstall={onUninstall} interactive />;

  return createPortal(
    <div className="detail-layer">
      <div className="detail-catcher" onClick={close} aria-hidden="true" />
      <div ref={measure} className="detail-measure" style={{ width }} aria-hidden="true">
        <DetailBody item={item} instances={instances} brewInstalled={brewInstalled} brewVersion={brewVersion} brewPath={brewPath} homebrewReady={homebrewReady} task={task} result={result} actionError={actionError} confirm={confirm} onConfirm={setConfirm} onInstall={onInstall} onUninstall={onUninstall} />
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
