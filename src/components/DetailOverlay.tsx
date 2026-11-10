import { useCallback, useEffect, useRef, useState } from "react";
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

function cardRect() {
  const width = Math.min(720, Math.max(320, window.innerWidth - 40));
  const height = Math.min(500, Math.max(340, window.innerHeight - 80));
  return {
    left: Math.max(20, (window.innerWidth - width) / 2),
    top: Math.max(32, (window.innerHeight - height) / 2),
    width,
    height
  };
}

export function DetailOverlay({ item, origin, onClose }: { item: Software; origin: OrbOrigin; onClose: () => void }) {
  const reduced = useReducedMotion();
  const [entered, setEntered] = useState(reduced);
  const [target] = useState(cardRect);
  const closeButton = useRef<HTMLButtonElement>(null);
  const timer = useRef<number | null>(null);

  const close = useCallback(() => {
    if (timer.current !== null) return;
    if (reduced) { onClose(); return; }
    setEntered(false);
    timer.current = window.setTimeout(onClose, 420);
  }, [onClose, reduced]);

  useEffect(() => {
    if (reduced) { setEntered(true); return; }
    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => setEntered(true));
    });
    return () => {
      cancelAnimationFrame(first);
      if (second) cancelAnimationFrame(second);
    };
  }, [reduced]);

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
    ? { left: target.left, top: target.top, width: target.width, height: target.height, borderRadius: 28 }
    : { left: origin.left, top: origin.top, width: origin.width, height: origin.height, borderRadius: Math.max(1, origin.width / 2) };

  return createPortal(
    <div className="detail-layer">
      <div className={`detail-backdrop${entered ? " open" : ""}`} onClick={close} aria-hidden="true" />
      <section className={`morph-card${entered ? " open" : ""}`} style={geometry} role="dialog" aria-modal="true" aria-label={`${item.name} 详情`} onClick={event => event.stopPropagation()}>
        <div className="detail-inner">
          <button ref={closeButton} type="button" className="detail-close" aria-label="关闭详情" onClick={close}>×</button>
          <header className="detail-head">
            <span className="detail-logo" style={{ "--orb-accent": item.accent } as CSSProperties}><SoftwareLogo id={item.id} name={item.name} /></span>
            <div>
              <span className="stage-pill">{STAGE_LABEL[item.stage]}</span>
              <h2>{item.name}</h2>
              <p>{item.description}</p>
            </div>
          </header>

          <div className="detail-status" role="status"><span className="status-dot" />尚未检测 · 第一阶段不读取本机状态</div>

          <div className="detail-fields">
            <label>版本
              <select disabled aria-label="选择版本"><option>版本列表将在第二阶段接入</option></select>
            </label>
            <div><span>计划来源</span><strong>{item.source}</strong></div>
            <div><span>安装路径</span><strong>—</strong></div>
          </div>

          <div className="detail-actions">
            <button type="button" className="primary" disabled>安装（第二阶段接入）</button>
            <button type="button" disabled>卸载</button>
          </div>
          <p className="detail-note">第一阶段只验证界面与交互。检测、下载、安装、卸载和环境配置均未接入，不会修改系统或旧版 DevHub。</p>

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
      </section>
    </div>,
    document.body
  );
}
