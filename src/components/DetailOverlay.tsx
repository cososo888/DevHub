import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Software } from "../catalog";
import { STAGE_LABEL } from "../catalog";
import { useReducedMotion } from "../hooks/useReducedMotion";
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

function DetailBody({ item }: { item: Software }) {
  return (
    <div className="detail-inner">
      <header className="detail-head">
        <span className="detail-logo" style={{ "--orb-accent": item.accent } as CSSProperties}><SoftwareLogo id={item.id} name={item.name} /></span>
        <div className="detail-title">
          <span className="stage-pill">{STAGE_LABEL[item.stage]}</span>
          <h2>{item.name}</h2>
          <p>{item.description}</p>
        </div>
      </header>

      <div className="detail-status" role="status"><span className="status-dot" />尚未检测 · 第一阶段不读取本机状态</div>

      <section className="detail-block">
        <h3>已安装版本</h3>
        <p className="detail-muted">未检测到本机版本。</p>
      </section>

      <section className="detail-block">
        <h3>可安装版本</h3>
        <select disabled aria-label="选择版本"><option>版本列表将在第二阶段接入</option></select>
      </section>

      <section className="detail-block">
        <h3>安装来源</h3>
        <p>{item.source}</p>
      </section>

      <div className="detail-actions">
        <button type="button" className="primary" disabled>安装（第二阶段接入）</button>
        <button type="button" disabled>卸载</button>
      </div>

      <p className="detail-note">第一阶段仅验证界面与交互；不会安装、卸载或修改系统。</p>

      <details className="detail-advanced">
        <summary>高级详情</summary>
        <dl>
          <dt>软件标识</dt><dd>{item.id}</dd>
          <dt>分类</dt><dd>{item.category}</dd>
          <dt>计划安装机制</dt><dd>{item.source}</dd>
          <dt>阶段</dt><dd>{STAGE_LABEL[item.stage]}</dd>
        </dl>
      </details>
    </div>
  );
}

export function DetailOverlay({ item, origin, onClose }: { item: Software; origin: OrbOrigin; onClose: () => void }) {
  const reduced = useReducedMotion();
  const [width] = useState(() => Math.min(CARD_WIDTH, window.innerWidth - PAGE_PADDING * 2));
  const [target, setTarget] = useState(() => anchoredCard(origin, width, CARD_MIN_HEIGHT));
  const [entered, setEntered] = useState(false);
  const measure = useRef<HTMLDivElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
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
  }, [origin.left, origin.top, origin.width, origin.height, width, reduced]);

  useEffect(() => {
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    closeButton.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); close(); }
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
      <div ref={measure} className="detail-measure" style={{ width }} aria-hidden="true"><DetailBody item={item} /></div>
      <section className={`morph-card${entered ? " open" : ""}`} style={geometry} role="dialog" aria-modal="true" aria-label={`${item.name} 详情`} onClick={event => event.stopPropagation()}>
        <button ref={closeButton} type="button" className="detail-close" aria-label="关闭详情" onClick={close}>×</button>
        <DetailBody item={item} />
      </section>
    </div>,
    document.querySelector(".app") ?? document.body
  );
}
