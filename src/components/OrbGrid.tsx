import type { Software } from "../catalog";
import { Orb } from "./Orb";
import type { OrbOrigin } from "./DetailOverlay";

export function OrbGrid({ items, installed, installing = new Set<string>(), sourceId = null, onOpen }: { items: Software[]; installed: ReadonlySet<string>; installing?: ReadonlySet<string>; sourceId?: string | null; onOpen: (item: Software, origin: OrbOrigin) => void }) {
  return (
    <div className="orb-grid" aria-label="软件列表">
      {items.map(item => <Orb key={item.id} item={item} installed={installed.has(item.id)} installing={installing.has(item.id)} source={sourceId === item.id} onOpen={onOpen} />)}
    </div>
  );
}
