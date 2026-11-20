import { useEffect, useMemo, useRef, useState } from "react";
import { SOFTWARE } from "./catalog";
import type { Software } from "./catalog";
import { DetailOverlay } from "./components/DetailOverlay";
import type { OrbOrigin } from "./components/DetailOverlay";
import { OrbGrid } from "./components/OrbGrid";
import { InstallationSidebar } from "./components/InstallationSidebar";
import type { InstallationFilter } from "./components/InstallationSidebar";
import { Toolbar } from "./components/Toolbar";
import { useDetection } from "./hooks/useDetection";
import { useInstallTask } from "./hooks/useInstallTask";
import { useTheme } from "./hooks/useTheme";
import { desktop } from "./lib/ipc";
import { filterCurated } from "./lib/catalogFilter";
import type { HomebrewResult, InstalledPackages, PackageKind, PackageVersion, SoftwareInstance } from "./lib/ipc";

function formatUpdated(value: number | null | undefined) {
  if (!value) return "尚未检测";
  return new Date(value).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
}

function packageKey(kind: PackageKind, packageName: string) {
  return `${kind}:${packageName}`;
}

function remoteSoftware(result: HomebrewResult): Software {
  const safe = result.package.replace(/[^a-zA-Z0-9]+/g, "-").replace(/^-|-$/g, "");
  return {
    id: `brew-${result.kind}-${safe}`,
    name: result.name || result.package,
    category: "Homebrew 搜索",
    description: result.description || "Homebrew 软件包",
    popularity: 0,
    accent: "#6B7280",
    source: result.kind === "cask" ? "Homebrew cask" : "Homebrew formula",
    stage: "planned",
    package: result.package,
    kind: result.kind
  };
}

export default function App() {
  const { theme, setTheme } = useTheme();
  const { snapshot, refreshing, error, refresh } = useDetection();
  const install = useInstallTask();
  const [query, setQuery] = useState("");
  const [installationFilter, setInstallationFilter] = useState<InstallationFilter>("all");
  const [packagesStatus, setPackagesStatus] = useState<"loading" | "ready" | "error">("loading");
  const [category, setCategory] = useState("全部");
  const [searchOpen, setSearchOpen] = useState(false);
  const [selected, setSelected] = useState<{ item: Software; origin: OrbOrigin } | null>(null);
  const [packages, setPackages] = useState<InstalledPackages>({ formulae: [], casks: [] });
  const [remote, setRemote] = useState<HomebrewResult[]>([]);
  const [remoteQuery, setRemoteQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [searchRetry, setSearchRetry] = useState(0);
  const searchCache = useRef(new Map<string, HomebrewResult[]>());

  useEffect(() => {
    let live = true;
    setPackagesStatus("loading");
    desktop<InstalledPackages>("get_installed_packages")
      .then(value => { if (live) { setPackages(value); setPackagesStatus("ready"); } })
      .catch(() => { if (live) setPackagesStatus("error"); });
    return () => { live = false; };
  }, [snapshot?.refreshedAt, install.result?.jobId]);

  useEffect(() => {
    const text = query.trim();
    if (text.length < 2) {
      setRemote([]);
      setSearching(false);
      setSearchError(null);
      return;
    }
    const key = text.toLowerCase();
    const cached = searchCache.current.get(key);
    if (cached) {
      setRemote(cached);
      setRemoteQuery(key);
      setSearching(false);
      setSearchError(null);
      return;
    }
    let active = true;
    setSearching(true);
    setSearchError(null);
    const timer = window.setTimeout(() => {
      desktop<HomebrewResult[]>("search_homebrew", { query: text })
        .then(results => {
          if (!active) return;
          searchCache.current.set(key, results);
          setRemote(results);
          setRemoteQuery(key);
        })
        .catch(value => { if (active) { setRemote([]); setSearchError(String(value)); } })
        .finally(() => { if (active) setSearching(false); });
    }, 320);
    return () => { active = false; window.clearTimeout(timer); };
  }, [query, searchRetry]);

  const instancesBySoftware = useMemo(() => {
    const map = new Map<string, SoftwareInstance[]>();
    for (const instance of snapshot?.instances ?? []) {
      const list = map.get(instance.softwareId) ?? [];
      list.push(instance);
      map.set(instance.softwareId, list);
    }
    return map;
  }, [snapshot]);

  const brewMap = useMemo(() => {
    const map = new Map<string, PackageVersion>();
    for (const item of packages.formulae) map.set(packageKey("formula", item.package), item);
    for (const item of packages.casks) map.set(packageKey("cask", item.package), item);
    return map;
  }, [packages]);

  const isInstalled = (item: Software) => instancesBySoftware.has(item.id) || brewMap.has(packageKey(item.kind, item.package));

  const globalSearch = query.trim().length >= 2;
  const curated = useMemo(() => {
    return filterCurated(SOFTWARE, query, category)
      .sort((a, b) => b.popularity - a.popularity);
  }, [category, query]);

  const remoteItems = useMemo(() => {
    if (query.trim().length < 2 || remoteQuery !== query.trim().toLowerCase()) return [];
    const seen = new Set(SOFTWARE.map(item => packageKey(item.kind, item.package)));
    const values: Software[] = [];
    for (const result of remote) {
      const key = packageKey(result.kind, result.package);
      if (seen.has(key)) continue;
      seen.add(key);
      values.push(remoteSoftware(result));
    }
    return values.sort((a, b) => a.name.localeCompare(b.name));
  }, [query, remote, remoteQuery]);

  const candidates = useMemo(() => [...curated, ...remoteItems], [curated, remoteItems]);
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
  const searchCount = remoteItems.length;
  const selectedInstances = selected ? instancesBySoftware.get(selected.item.id) ?? [] : [];
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
      <div className="workspace">
      <InstallationSidebar value={installationFilter} onChange={setInstallationFilter} counts={counts} />
      <div className="content-pane">
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
        {globalSearch && (
          <div className="search-status" role="status">
            {searching
              ? `正在搜索“${query.trim()}”…`
              : searchError
                ? <><span>Homebrew 搜索失败：{searchError}</span><button type="button" onClick={() => setSearchRetry(value => value + 1)}>重试</button></>
                : <span>“{query.trim()}” · 精选 {curated.length} 款 · 更多软件 {searchCount} 款</span>}
            <button type="button" onClick={() => setQuery("")}>清除关键词</button>
          </div>
        )}
        {filterNotice && <p className="detection-line" role="status">{filterNotice}</p>}
        {items.length > 0
          ? <OrbGrid items={items} installed={installedIds} installing={busyIds} sourceId={selected?.item.id ?? null} onOpen={(item, origin) => setSelected({ item, origin })} />
          : !filterNotice && <div className="empty">{searching ? "正在搜索…" : installationFilter === "installed" ? "当前范围内没有已安装的软件。" : installationFilter === "not-installed" ? "当前检测范围内没有未安装的软件。" : "没有匹配的软件。"}</div>}
      </main>
      </div>
      </div>
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
