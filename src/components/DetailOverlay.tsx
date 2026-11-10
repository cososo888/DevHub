import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Software } from "../catalog";
import { useReducedMotion } from "../hooks/useReducedMotion";
import type { InstallFinished, InstallProgress, VersionRow } from "../lib/ipc";
import { SoftwareLogo } from "./SoftwareLogo";

export interface OrbOrigin {
  left: number;
  top: number;
  width: number;
  height: number;
}

type Confirm = { version: string; action: "install" | "uninstall" } | null;

const EMPTY_VERSIONS: VersionRow[] = [];
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

function DetailBody({ item, versions, loading, error, task, result, actionError, confirm, onRetry, onConfirm, onInstall, onUninstall, interactive = false }: {
  item: Software;
  versions: VersionRow[];
  loading: boolean;
  error: string | null;
  task: InstallProgress | null;
  result: InstallFinished | null;
  actionError: string | null;
  confirm: Confirm;
  onRetry: () => void;
  onConfirm: (value: Confirm) => void;
  onInstall: (version: string) => void;
  onUninstall: (version: string, formula: string | null) => void;
  interactive?: boolean;
}) {
  return (
    <div className="detail-inner" tabIndex={interactive ? 0 : undefined} role={interactive ? "region" : undefined} aria-label={interactive ? `${item.name} 详情内容` : undefined}>
      <header className="detail-head">
        <h2>{item.name}</h2>
        <p>{item.description}</p>
      </header>

      <div className="version-list">
        {loading && <p className="versions-message">正在查询版本…</p>}
        {!loading && versions.length === 0 && (
          <div className="versions-empty">
            <span>{error ? "版本查询失败" : "暂无可安装版本"}</span>
            <button type="button" onClick={onRetry}>重试</button>
          </div>
        )}
        {versions.map(row => {
          const rowTask = task?.version === row.version ? task : null;
          const rowResult = result?.version === row.version ? result : null;
          const confirming = confirm?.version === row.version;
          return (
            <div className={`version-row${row.installed ? " installed" : ""}`} key={`${row.version}:${row.formula ?? ""}`}>
              <div className="version-main">
                <span className="version-number">{row.version}</span>
                <div className="version-actions">
                  {row.installed && <span className="installed-label">已安装</span>}
                  {!row.installed && row.installable && !confirming && (
                    <button type="button" aria-label={`安装 ${row.version}`} disabled={Boolean(task) || Boolean(confirm)} onClick={() => onConfirm({ version: row.version, action: "install" })}>安装</button>
                  )}
                  {row.installed && row.canUninstall && !confirming && (
                    <button type="button" aria-label={`卸载 ${row.version}`} disabled={Boolean(task) || Boolean(confirm)} onClick={() => onConfirm({ version: row.version, action: "uninstall" })}>卸载</button>
                  )}
                  {!row.installed && !row.installable && <span className="unavailable-label">不可安装</span>}
                </div>
              </div>
              {confirming && confirm && (
                <div className="row-confirm">
                  <span>{confirm.action === "install" ? `安装 ${row.version}？` : `卸载 ${row.version}？`}</span>
                  {confirm.action === "uninstall" && <small>其他来源的同名版本不会被修改。</small>}
                  <div className="row-confirm-actions">
                    <button type="button" className="primary" onClick={() => { onConfirm(null); confirm.action === "install" ? onInstall(row.version) : onUninstall(row.version, row.formula); }}>确认</button>
                    <button type="button" onClick={() => onConfirm(null)}>取消</button>
                  </div>
                </div>
              )}
              {rowTask && (
                <div className="row-progress" role="status">
                  <span className="mini-spinner" />
                  <span>{rowTask.message}</span>
                  <div className="indeterminate" />
                </div>
              )}
              {rowResult && !rowResult.success && (
                <div className="row-error">{rowResult.message}</div>
              )}
            </div>
          );
        })}
        {error && versions.length > 0 && <p className="versions-error">{error}</p>}
        {actionError && <p className="versions-error">{actionError}</p>}
      </div>
    </div>
  );
}

export function DetailOverlay({ item, origin, onClose, versions = EMPTY_VERSIONS, loading = false, error = null, onRetry = () => {}, task = null, result = null, actionError = null, onInstall, onUninstall }: {
  item: Software;
  origin: OrbOrigin;
  onClose: () => void;
  versions?: VersionRow[];
  loading?: boolean;
  error?: string | null;
  onRetry?: () => void;
  task?: InstallProgress | null;
  result?: InstallFinished | null;
  actionError?: string | null;
  onInstall: (version: string) => void;
  onUninstall: (version: string, formula: string | null) => void;
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
  const versionSignature = versions.map(row => `${row.version}:${row.installed}:${row.installable}:${row.canUninstall}`).join("|");

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
  }, [origin.left, origin.top, origin.width, origin.height, width, viewport.width, viewport.height, reduced, versionSignature, loading, error, task?.message, result, actionError, confirm]);

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
  const body = <DetailBody item={item} versions={versions} loading={loading} error={error} task={task} result={result} actionError={actionError} confirm={confirm} onRetry={onRetry} onConfirm={setConfirm} onInstall={onInstall} onUninstall={onUninstall} interactive />;

  return createPortal(
    <div className="detail-layer">
      <div className="detail-catcher" onClick={close} aria-hidden="true" />
      <div ref={measure} className="detail-measure" style={{ width }} aria-hidden="true">
        <DetailBody item={item} versions={versions} loading={loading} error={error} task={task} result={result} actionError={actionError} confirm={confirm} onRetry={onRetry} onConfirm={setConfirm} onInstall={onInstall} onUninstall={onUninstall} />
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
