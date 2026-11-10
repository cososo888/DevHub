import type { CSSProperties } from "react";
import type { Software } from "../catalog";
import { SoftwareLogo } from "./SoftwareLogo";
import type { OrbOrigin } from "./DetailOverlay";

export function Orb({ item, installed = false, onOpen }: { item: Software; installed?: boolean; onOpen: (item: Software, origin: OrbOrigin) => void }) {
  return (
    <button
      type="button"
      className={`orb${installed ? " is-installed" : ""}`}
      aria-label={item.name}
      onClick={event => onOpen(item, event.currentTarget.getBoundingClientRect())}
    >
      <span className="orb-circle" style={{ "--orb-accent": item.accent } as CSSProperties}>
        <SoftwareLogo id={item.id} name={item.name} />
      </span>
      <span className="orb-name">{item.name}</span>
    </button>
  );
}
