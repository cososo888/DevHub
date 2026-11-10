import { useRef } from "react";
import type { CSSProperties } from "react";
import type { Software } from "../catalog";
import { SoftwareLogo } from "./SoftwareLogo";
import type { OrbOrigin } from "./DetailOverlay";

export function Orb({ item, installed = false, installing = false, source = false, onOpen }: { item: Software; installed?: boolean; installing?: boolean; source?: boolean; onOpen: (item: Software, origin: OrbOrigin) => void }) {
  const circle = useRef<HTMLSpanElement>(null);
  return (
    <button
      type="button"
      className={`orb${installed ? " is-installed" : ""}${installing ? " is-installing" : ""}${source ? " is-source" : ""}`}
      aria-label={item.name}
      aria-busy={installing || undefined}
      aria-hidden={source || undefined}
      tabIndex={source ? -1 : undefined}
      onClick={event => onOpen(item, (circle.current ?? event.currentTarget).getBoundingClientRect())}
    >
      <span ref={circle} className="orb-circle" style={{ "--orb-accent": item.accent } as CSSProperties}>
        <SoftwareLogo id={item.id} name={item.name} />
      </span>
      <span className="orb-name">{item.name}</span>
    </button>
  );
}
