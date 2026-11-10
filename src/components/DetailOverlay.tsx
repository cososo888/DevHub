import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Software } from "../catalog";
import { STAGE_LABEL } from "../catalog";
import { useReducedMotion } from "../hooks/useReducedMotion";
import type { DetectionError, Health, HomebrewStatus, SoftwareInstance } from "../lib/ipc";
import { SoftwareLogo } from "./SoftwareLogo";

export interface OrbOrigin {
  left: number;
  top: number;
  width: number;
  height: number;
}

const CARD_WIDTH = 380;
const CARD_MIN_HEIGHT = 360;
const PAGE_PADDING = 16;
const RADIUS = 24;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

/**
 * The orb itself is one corner of the expanded rectangle. The card grows toward the
 * available side of the window instead of moving to the screen center.
 */
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
  if (natural <= below) {
    top = origin.top;
  } else if (natural <= above) {
    top = origin.top + origin.height - natural;
  } else {
    // Neither side can hold the natural card: keep the full content visible and
    // clamp to the viewport instead of shrinking the card into a scroll box.
    top = clamp(origin.top, PAGE_PADDING, viewportHeight - natural - PAGE_PADDING);
  }
  top = clamp(top, PAGE_PADDING, viewportHeight - natural - PAGE_PADDING);
  return { left, top, width: w, height: natural };
}

const SOURCE_LABEL: Record<string, string> = {
  path: "PATH",
  homebrewFormula: "Homebrew formula",
  homebrewCask: "Homebrew cask",
  devhubV1: "旧版 DevHub",
  devhubV2: "DevHub V2",
  officialApp: "官方应用"
};

const HEALTH_LABEL: Record<Health, string> = {
  ok: "正常",
  unverified: "未验证",
  broken: "异常"
};

function formatTime(value: number | null | undefined) {
  if (!value) return "尚未检测";
  return new Date(value).toLocaleString("zh-CN", { hour12: false });
}

function sourceLabels(instances: SoftwareInstance[]) {
  const labels = instances.map(instance => SOURCE_LABEL[instance.source] ?? instance.source);
  return [...new Set(labels)];
}

function DetailBody({ item, instances, detected, refreshing, stale, homebrew, errors, refreshedAt, interactive = false }: {
  item: Software;
  instances: SoftwareInstance[];
  detected: boolean;
  refreshing: boolean;
  stale: boolean;
  homebrew: HomebrewStatus | null | undefined;
  errors: DetectionError[];
  refreshedAt: number | null;
  interactive?: boolean;
}) {
  const installed = instances.length > 0;
  const allUnverified = installed && instances.every(instance => instance.health !== "ok");
  const anyBroken = installed && instances.some(instance => instance.health === "broken");
  const status = installed
    ? allUnverified
      ? `已安装 · ${instances.length} 个版本（未验证）`
      : anyBroken
        ? `已安装 · ${instances.length} 个版本（部分异常）`
        : `已安装 · ${instances.length} 个版本`
    : refreshing ? "正在检测…" : detected ? "未安装" : "尚未检测";
  const detectionNote = stale
    ? "缓存结果 · 正在重新验证"
    : refreshing
      ? "正在读取本机环境…"
      : `最后检测：${formatTime(refreshedAt)}`;

  return (
    <div className="detail-inner" tabIndex={interactive ? 0 : undefined} role={interactive ? "region" : undefined} aria-label={interactive ? `${item.name} 详情内容` : undefined}>
      <header className="detail-head">
        <span className="detail-logo" style={{ "--orb-accent": item.accent } as CSSProperties}><SoftwareLogo id={item.id} name={item.name} /></span>
        <div className="detail-title">
          <span className="stage-pill">{STAGE_LABEL[item.stage]}</span>
          <h2>{item.name}</h2>
          <p>{item.description}</p>
        </div>
      </header>

      <div className="detail-status" role="status"><span className={`status-dot ${installed ? "installed" : ""}`} />{status}</div>

      <section className="detail-block">
        <h3>已安装版本</h3>
        {installed ? (
          <ul className="instance-list">
            {instances.map(instance => (
              <li key={`${instance.path}:${instance.version ?? "unknown"}`}>
                <div className="instance-head">
                  <strong>{instance.version ?? "版本未知"}</strong>
                  <span className={`health-badge ${instance.health}`}>{HEALTH_LABEL[instance.health]}</span>
                </div>
                <div className="instance-meta">
                  {SOURCE_LABEL[instance.source] ?? instance.source}
                  {instance.managedBy === "devhub" ? " · DevHub 管理" : ""}
                  {instance.kind === "app" ? " · 应用" : " · CLI"}
                </div>
                <code className="instance-path">{instance.path}</code>
                {instance.detail ? <p className="instance-detail">{instance.detail}</p> : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="detail-muted">{refreshing ? "正在检测本机版本…" : detected ? "未检测到本机安装。" : "尚未检测。"}</p>
        )}
      </section>

      <section className="detail-block">
        <h3>可安装版本</h3>
        <select disabled aria-label="选择版本"><option>版本列表将在第三阶段接入</option></select>
      </section>

      <section className="detail-block">
        <h3>安装来源</h3>
        <p>{installed ? sourceLabels(instances).join("、") : item.source}</p>
      </section>

      <div className="detail-actions">
        <button type="button" className="primary" disabled>安装（第三阶段接入）</button>
        <button type="button" disabled>卸载</button>
      </div>

      <p className="detail-note">第二阶段仅检测本机环境；不会安装、卸载或修改系统。</p>

      <details className="detail-advanced">
        <summary>高级详情</summary>
        <dl>
          <dt>软件标识</dt><dd>{item.id}</dd>
          <dt>分类</dt><dd>{item.category}</dd>
          <dt>计划安装机制</dt><dd>{item.source}</dd>
          <dt>最后检测</dt><dd>{formatTime(refreshedAt)}</dd>
          <dt>Homebrew</dt><dd>{homebrew ? homebrew.available ? `${homebrew.version ?? "已安装"} · ${homebrew.prefix ?? "前缀未知"}` : "未检测到" : "尚未检测"}</dd>
          <dt>缓存状态</dt><dd>{stale ? "缓存结果，尚未重新验证" : "本次检测"}</dd>
          <dt>检测说明</dt><dd>{detectionNote}</dd>
        </dl>
        {errors.length > 0 && (
          <div className="detection-errors">
            <strong>检测警告</strong>
            {errors.map(error => <p key={`${error.source}:${error.message}`}>{error.source}：{error.message}</p>)}
          </div>
        )}
      </details>
    </div>
  );
}

export function DetailOverlay({ item, origin, onClose, instances = [], detected = false, refreshing = false, stale = false, homebrew = null, errors = [], refreshedAt = null }: {
  item: Software;
  origin: OrbOrigin;
  onClose: () => void;
  instances?: SoftwareInstance[];
  detected?: boolean;
  refreshing?: boolean;
  stale?: boolean;
  homebrew?: HomebrewStatus | null;
  errors?: DetectionError[];
  refreshedAt?: number | null;
}) {
  const reduced = useReducedMotion();
  const [width] = useState(() => Math.min(CARD_WIDTH, window.innerWidth - PAGE_PADDING * 2));
  const [target, setTarget] = useState(() => anchoredCard(origin, width, CARD_MIN_HEIGHT));
  const [entered, setEntered] = useState(false);
  const measure = useRef<HTMLDivElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const card = useRef<HTMLElement>(null);
  const timer = useRef<number | null>(null);

  const close = useCallback(() => {
    if (timer.current !== null) return;
    if (reduced) { onClose(); return; }
    setEntered(false);
    timer.current = window.setTimeout(onClose, 420);
  }, [onClose, reduced]);

  useLayoutEffect(() => {
    const natural = Math.max(measure.current?.scrollHeight ?? 0, CARD_MIN_HEIGHT);
    setTarget(anchoredCard(origin, width, natural));
    if (reduced) { setEntered(true); return; }
    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => setEntered(true));
    });
    return () => {
      cancelAnimationFrame(first);
      if (second) cancelAnimationFrame(second);
    };
  }, [origin.left, origin.top, origin.width, origin.height, width, reduced, instances.length, detected, refreshing, stale, instances.map(instance => `${instance.path}:${instance.version}:${instance.health}`).join("|")]);

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
      if (event.key === "PageDown" || event.key === "ArrowDown") {
        content.scrollBy({ top: event.key === "PageDown" ? page : 40 });
        event.preventDefault();
      } else if (event.key === "PageUp" || event.key === "ArrowUp") {
        content.scrollBy({ top: event.key === "PageUp" ? -page : -40 });
        event.preventDefault();
      } else if (event.key === "Home") {
        content.scrollTo({ top: 0 });
        event.preventDefault();
      } else if (event.key === "End") {
        content.scrollTo({ top: content.scrollHeight });
        event.preventDefault();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = previous;
      window.removeEventListener("keydown", onKey);
    };
  }, [close]);

  useEffect(() => () => {
    if (timer.current !== null) window.clearTimeout(timer.current);
  }, []);

  const geometry: CSSProperties = entered
    ? { left: target.left, top: target.top, width: target.width, height: target.height, borderRadius: RADIUS }
    : { left: origin.left, top: origin.top, width: origin.width, height: origin.height, borderRadius: Math.max(1, origin.width / 2) };

  return createPortal(
    <div className="detail-layer">
      <div className="detail-catcher" onClick={close} aria-hidden="true" />
      <div ref={measure} className="detail-measure" style={{ width }} aria-hidden="true"><DetailBody item={item} instances={instances} detected={detected} refreshing={refreshing} stale={stale} homebrew={homebrew} errors={errors} refreshedAt={refreshedAt} /></div>
      <section ref={card} className={`morph-card${entered ? " open" : ""}`} style={geometry} role="dialog" aria-modal="true" aria-label={`${item.name} 详情`} onClick={event => event.stopPropagation()}>
        <button ref={closeButton} type="button" className="detail-close" aria-label="关闭详情" onClick={close}>×</button>
        <DetailBody item={item} instances={instances} detected={detected} refreshing={refreshing} stale={stale} homebrew={homebrew} errors={errors} refreshedAt={refreshedAt} interactive />
      </section>
    </div>,
    document.querySelector(".app") ?? document.body
  );
}
