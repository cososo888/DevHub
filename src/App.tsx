import { useEffect, useMemo, useState } from "react";
import { SOFTWARE } from "./catalog";
import type { Software } from "./catalog";
import { DetailOverlay } from "./components/DetailOverlay";
import type { OrbOrigin } from "./components/DetailOverlay";
import { OrbGrid } from "./components/OrbGrid";
import { Toolbar } from "./components/Toolbar";
import { useDetection } from "./hooks/useDetection";
import { useInstallTask } from "./hooks/useInstallTask";
import { useSoftwareVersions } from "./hooks/useSoftwareVersions";
import { useTheme } from "./hooks/useTheme";
import { desktop } from "./lib/ipc";
import type { AppStatus, SoftwareInstance } from "./lib/ipc";

function formatUpdated(value: number | null | undefined) {
  if (!value) return "尚未检测";
  return new Date(value).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
}

export default function App() {
  const { theme, setTheme } = useTheme();
  const { snapshot, refreshing, error, refresh } = useDetection();
  const install = useInstallTask();
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

  const instancesBySoftware = useMemo(() => {
    const map = new Map<string, SoftwareInstance[]>();
    for (const instance of snapshot?.instances ?? []) {
      const list = map.get(instance.softwareId) ?? [];
      list.push(instance);
      map.set(instance.softwareId, list);
    }
    return map;
  }, [snapshot]);

  const installed = useMemo(() => new Set(instancesBySoftware.keys()), [instancesBySoftware]);

  const items = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return SOFTWARE
      .filter(item => (category === "全部" || item.category === category) && (!needle || item.name.toLowerCase().includes(needle)))
      .sort((a, b) => Number(installed.has(b.id)) - Number(installed.has(a.id)) || b.popularity - a.popularity);
  }, [category, query, installed]);

  const selectedId = selected?.item.id ?? null;
  const versions = useSoftwareVersions(selectedId, `${install.result?.jobId ?? 0}:${snapshot?.refreshedAt ?? 0}`);
  const busyIds = useMemo(() => new Set(install.task ? [install.task.softwareId] : []), [install.task]);
  const instanceCount = snapshot?.instances?.length ?? 0;
  const cacheState = !snapshot
    ? refreshing ? "正在读取本机环境…" : "尚未检测"
    : snapshot.stale
      ? refreshing ? "缓存结果 · 正在重新验证" : "缓存结果已过期 · 等待刷新"
      : refreshing
        ? "正在刷新…"
        : `已检测 ${instanceCount} 个安装实例 · 更新于 ${formatUpdated(snapshot.refreshedAt)}`;
  const warning = snapshot?.errors?.length ? ` · ${snapshot.errors.length} 项检测警告` : "";

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
            <p className={`detection-line${error ? " error" : ""}`} role="status">
              {cacheState}
              {warning}
              {error ? ` · ${error}` : ""}
            </p>
          </div>
          <div className="heading-actions">
            <button type="button" className="refresh-button" aria-label="刷新检测" title="刷新检测" disabled={refreshing} onClick={() => void refresh()}>
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" className={refreshing ? "spin" : ""}>
                <path d="M20 11a8 8 0 1 0-2.3 5.7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                <path d="M20 5v6h-6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
            <span className="phase-pill">{status ? `Phase ${status.phase} · 安装` : "Phase 3 · 安装"}</span>
          </div>
        </div>
        {items.length > 0
          ? <OrbGrid items={items} installed={installed} installing={busyIds} sourceId={selectedId} onOpen={(item, origin) => setSelected({ item, origin })} />
          : <div className="empty">没有匹配的软件，换个关键词或分类试试。</div>}
        <p className="footnote">
          {status ? `${status.product} · Phase ${status.phase}` : "DevHub V2 · Phase 3"} · 检测只读 · 安装需确认
        </p>
      </main>
      {selected && (
        <DetailOverlay
          item={selected.item}
          origin={selected.origin}
          versions={versions.catalog?.versions ?? []}
          loading={versions.loading}
          error={versions.error}
          onRetry={versions.retry}
          task={install.task && install.task.softwareId === selectedId ? install.task : null}
          result={install.result && install.result.softwareId === selectedId ? install.result : null}
          actionError={install.error}
          onInstall={(version) => { if (selected) void install.install(selected.item.id, version); }}
          onUninstall={(version, formula) => { if (selected) void install.uninstall(selected.item.id, version, formula); }}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
