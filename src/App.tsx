import { useEffect, useMemo, useState } from "react";
import { SOFTWARE } from "./catalog";
import type { Software } from "./catalog";
import { DetailOverlay } from "./components/DetailOverlay";
import type { OrbOrigin } from "./components/DetailOverlay";
import { OrbGrid } from "./components/OrbGrid";
import { Toolbar } from "./components/Toolbar";
import { useTheme } from "./hooks/useTheme";
import { desktop } from "./lib/ipc";
import type { AppStatus } from "./lib/ipc";

export default function App() {
  const { theme, setTheme } = useTheme();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("全部");
  const [searchOpen, setSearchOpen] = useState(false);
  const [selected, setSelected] = useState<{ item: Software; origin: OrbOrigin } | null>(null);
  const [status, setStatus] = useState<AppStatus | null>(null);

  useEffect(() => {
    let live = true;
    desktop<AppStatus>("get_app_status")
      .then(value => { if (live) setStatus(value); })
      .catch(() => { if (live) setStatus(null); });
    return () => { live = false; };
  }, []);

  const installed = useMemo(() => new Set<string>(), []);
  const items = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return SOFTWARE
      .filter(item => (category === "全部" || item.category === category) && (!needle || item.name.toLowerCase().includes(needle)))
      .sort((a, b) => Number(installed.has(b.id)) - Number(installed.has(a.id)) || b.popularity - a.popularity);
  }, [category, query, installed]);

  return (
    <div className="app" data-theme={theme}>
      <div className="ambient" aria-hidden="true" />
      <Toolbar
        query={query}
        onQuery={setQuery}
        category={category}
        onCategory={setCategory}
        searchOpen={searchOpen}
        onSearchToggle={() => setSearchOpen(open => !open)}
        theme={theme}
        onTheme={setTheme}
      />
      <main className="main">
        <div className="grid-heading">
          <div>
            <p className="eyebrow">DEVELOPMENT ENVIRONMENT</p>
            <h1>选择你的开发工具</h1>
            <p className="sub">点击圆形图标查看详情。第一阶段只验证界面与动效，不会安装或修改系统。</p>
          </div>
          <span className="phase-pill">Phase 1 · UI</span>
        </div>
        {items.length > 0
          ? <OrbGrid items={items} installed={installed} onOpen={(item, origin) => setSelected({ item, origin })} />
          : <div className="empty">没有匹配的软件，换个关键词或分类试试。</div>}
        <p className="footnote">{status ? `${status.product} · Phase ${status.phase}` : "DevHub V2 · Phase 1"} · 检测与安装将在第二阶段接入</p>
      </main>
      {selected && <DetailOverlay item={selected.item} origin={selected.origin} onClose={() => setSelected(null)} />}
    </div>
  );
}
