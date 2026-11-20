import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { SOFTWARE } from "./catalog";
import type { Software } from "./catalog";
import { DetailOverlay } from "./components/DetailOverlay";
import type { OrbOrigin } from "./components/DetailOverlay";
import { OrbGrid } from "./components/OrbGrid";
import { InstallationSidebar } from "./components/InstallationSidebar";
import type { InstallationFilter } from "./components/InstallationSidebar";
import { Toolbar } from "./components/Toolbar";
import { WindowChrome } from "./components/WindowChrome";
import { SearchPage, SearchToolbar } from "./components/SearchPage";
import { InstallationNotice } from "./components/InstallationNotice";
import { useSoftwareSearch } from "./hooks/useSoftwareSearch";
import { useDetection } from "./hooks/useDetection";
import { useInstallTask } from "./hooks/useInstallTask";
import { useTheme } from "./hooks/useTheme";
import { desktop } from "./lib/ipc";
import { filterCurated } from "./lib/catalogFilter";
import type { InstalledPackages, PackageKind, PackageVersion, SoftwareInstance } from "./lib/ipc";

function formatUpdated(value: number | null | undefined) {
  if (!value) return "尚未检测";
  return new Date(value).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
}

function packageKey(kind: PackageKind, packageName: string) {
  return `${kind}:${packageName}`;
}

export default function App() {
  const { theme, setTheme } = useTheme();
  const { snapshot, refreshing, error, refresh } = useDetection();
  const install = useInstallTask();
  const [query, setQuery] = useState("");
  const [installationFilter, setInstallationFilter] = useState<InstallationFilter>("all");
  const [packagesStatus, setPackagesStatus] = useState<"loading" | "ready" | "error">("loading");
  const [category, setCategory] = useState("全部");
  const [page, setPage] = useState<"library" | "search">("library");
  const [selected, setSelected] = useState<{ item: Software; origin: OrbOrigin } | null>(null);
  const [packages, setPackages] = useState<InstalledPackages>({ formulae: [], casks: [] });
  const search = useSoftwareSearch(query, page === "search");
  const scrollPositions = useRef({ library: 0, search: 0 });
  const navigate = useCallback((next: "library" | "search") => {
    if (next === page) return;
    scrollPositions.current[page] = window.scrollY;
    setSelected(null);
    setPage(next);
  }, [page]);

  useLayoutEffect(() => { window.scrollTo(0, scrollPositions.current[page]); }, [page]);
  useEffect(() => {
    if (page !== "search" || selected) return;
    const back = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) { event.preventDefault(); navigate("library"); }
    };
    window.addEventListener("keydown", back);
    return () => window.removeEventListener("keydown", back);
  }, [page, selected, navigate]);

  useEffect(() => {
    let live = true;
    setPackagesStatus("loading");
    desktop<InstalledPackages>("get_installed_packages")
      .then(value => { if (live) { setPackages(value); setPackagesStatus("ready"); } })
      .catch(() => { if (live) setPackagesStatus("error"); });
    return () => { live = false; };
  }, [snapshot?.refreshedAt, install.result?.jobId]);

  const instancesBySoftware = useMemo(() => {
    const map = new Map<string, SoftwareInstance[]>();
    for (const instance of snapshot?.instances ?? []) {
      const list = map.get(instance.softwareId) ?? [];
      list.push(instance);
      map.set(instance.softwareId, list);
    }
    for (const app of packages.externalApps ?? []) {
      const software = SOFTWARE.find(item => item.kind === "cask" && item.package === app.package);
      if (!software) continue;
      const list = map.get(software.id) ?? [];
      if (!list.some(instance => instance.path === app.path)) {
        list.push({ softwareId: software.id, version: null, source: "externalApp", sources: ["externalApp"],
          path: app.path, kind: "app", executable: false, managedBy: "external", health: "unverified",
          detail: null, lastDetectedAt: snapshot?.refreshedAt ?? 0 });
      }
      map.set(software.id, list);
    }
    return map;
  }, [snapshot, packages.externalApps]);

  const brewMap = useMemo(() => {
    const map = new Map<string, PackageVersion>();
    for (const item of packages.formulae) map.set(packageKey("formula", item.package), item);
    for (const item of packages.casks) map.set(packageKey("cask", item.package), item);
    return map;
  }, [packages]);

  const isInstalled = (item: Software) => (instancesBySoftware.get(item.id) ?? []).some(instance => item.kind !== "cask" || instance.kind === "app") || brewMap.has(packageKey(item.kind, item.package));

  const candidates = useMemo(() => filterCurated(SOFTWARE, "", category)
    .sort((a, b) => b.popularity - a.popularity), [category]);
  // Absence is meaningful only after both current detection and package queries succeeded.
  const absenceConfirmed = Boolean(snapshot && !snapshot.stale && !refreshing && !error
    && snapshot.errors.length === 0 && packagesStatus === "ready");
  const installedCount = candidates.filter(isInstalled).length;
  const counts = { all: candidates.length, installed: installedCount,
    "not-installed": absenceConfirmed ? candidates.length - installedCount : null };
  const items = candidates.filter(item => installationFilter === "all"
    || (installationFilter === "installed" ? isInstalled(item) : absenceConfirmed && !isInstalled(item)));
  const filterNotice = installationFilter === "not-installed" && !absenceConfirmed
    ? packagesStatus === "error" ? "软件状态查询未完成，暂不能确认未安装列表。"
      : refreshing || packagesStatus === "loading" ? "正在核对安装状态…" : "检测结果尚未确认，请刷新检测。"
    : null;
  const installedIds = useMemo(() => new Set(items.filter(isInstalled).map(item => item.id)), [items, instancesBySoftware, brewMap]);
  const selectedInstances = selected ? (instancesBySoftware.get(selected.item.id) ?? []).filter(instance => selected.item.kind !== "cask" || instance.kind === "app") : [];
  const selectedBrew = selected ? brewMap.get(packageKey(selected.item.kind, selected.item.package)) ?? null : null;
  const busyIds = useMemo(() => {
    if (!install.task) return new Set<string>();
    return new Set(items.filter(item => item.package === install.task?.softwareId).map(item => item.id));
  }, [install.task, items]);
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
      <WindowChrome>
        {page === "library"
          ? <Toolbar category={category} onCategory={setCategory} onSearch={() => navigate("search")} theme={theme} onTheme={setTheme}/>
          : <SearchToolbar query={query} onQuery={setQuery} onBack={() => navigate("library")} onSearch={search.retry} theme={theme} onTheme={setTheme}/>}
      </WindowChrome>
      <div className="workspace" hidden={page !== "library"}>
      <InstallationSidebar value={installationFilter} onChange={setInstallationFilter} counts={counts} />
      <div className="content-pane">
      <main className="main">
        <div className="grid-heading">
          <div className="heading-actions">
            <p className={`detection-line${error ? " error" : ""}`} role="status">
              {cacheState}
              {warning}
              {packagesStatus === "error" ? " · 部分软件包状态未确认" : ""}
              {error ? ` · ${error}` : ""}
            </p>
            <button type="button" className="refresh-button" aria-label="刷新检测" title="刷新检测" disabled={refreshing} onClick={() => void refresh()}>
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" className={refreshing ? "spin" : ""}>
                <path d="M20 11a8 8 0 1 0-2.3 5.7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                <path d="M20 5v6h-6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
          </div>
        </div>
        {snapshot && !snapshot.homebrew.available && (
          <div className="setup-hint" role="status">
            <span>安装功能需要 Homebrew。DevHub 不会静默获取管理员权限。</span>
            <button type="button" onClick={() => void desktop("open_homebrew_site").catch(() => {})}>查看安装方法</button>
            <button type="button" onClick={() => void refresh()}>重新检查</button>
          </div>
        )}
        {filterNotice && <p className="detection-line" role="status">{filterNotice}</p>}
        {items.length > 0
          ? <OrbGrid items={items} installed={installedIds} installing={busyIds} sourceId={selected?.item.id ?? null} onOpen={(item, origin) => setSelected({ item, origin })} />
          : !filterNotice && <div className="empty">{installationFilter === "installed" ? "当前范围内没有已安装的软件。" : installationFilter === "not-installed" ? "当前检测范围内没有未安装的软件。" : "没有匹配的软件。"}</div>}
      </main>
      </div>
      </div>
      {page === "search" && <SearchPage query={query} curated={search.curated} remote={search.remote} searching={search.searching} error={search.error}
        onQuery={setQuery} onRetry={search.retry} isInstalled={isInstalled} isBusy={item => install.task?.softwareId === item.package}
        sourceId={selected?.item.id ?? null} onOpen={(item, origin) => setSelected({ item, origin })}/>}
      <InstallationNotice result={install.result}/>
      {selected && (
        <DetailOverlay
          item={selected.item}
          origin={selected.origin}
          instances={selectedInstances}
          brewInstalled={Boolean(selectedBrew)}
          brewVersion={selectedBrew?.version ?? null}
          brewPath={selectedBrew?.path ?? null}
          homebrewReady={Boolean(snapshot?.homebrew.available)}
          task={install.task && install.task.softwareId === selected.item.package ? install.task : null}
          result={install.result && install.result.softwareId === selected.item.package ? install.result : null}
          actionError={install.errorPackage === selected.item.package && install.errorKind === selected.item.kind ? install.error : null}
          onInstall={() => void install.install(selected.item.package, selected.item.kind)}
          onUninstall={() => void install.uninstall(selected.item.package, selected.item.kind)}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
