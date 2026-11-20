import { useEffect, useMemo, useRef, useState } from "react";
import { SOFTWARE } from "../catalog";
import type { Software } from "../catalog";
import { filterCurated } from "../lib/catalogFilter";
import { desktop } from "../lib/ipc";
import type { HomebrewResult } from "../lib/ipc";

const packageKey = (item: { kind: string; package: string }) => `${item.kind}:${item.package}`;
const known = new Map(SOFTWARE.map(item => [packageKey(item), item]));

function remoteSoftware(result: HomebrewResult): Software {
  return known.get(packageKey(result)) ?? {
    id: `brew-${result.kind}-${result.package.replace(/[^a-zA-Z0-9]+/g, "-")}`,
    name: result.name || result.package,
    category: "Homebrew 搜索",
    description: result.description || "Homebrew 软件包",
    popularity: 0, accent: "#6B7280",
    source: result.kind === "cask" ? "Homebrew cask" : "Homebrew formula",
    stage: "planned", package: result.package, kind: result.kind
  };
}

type Response = { key: string; items: HomebrewResult[]; error: string | null; loading: boolean };

/** Search state lives outside the page, preserving results when returning to the library. */
export function useSoftwareSearch(query: string, active: boolean) {
  const key = query.trim().normalize("NFKC").toLowerCase();
  const [response, setResponse] = useState<Response>({ key: "", items: [], error: null, loading: false });
  const [attempt, setAttempt] = useState(0);
  const cache = useRef(new Map<string, HomebrewResult[]>());
  const canSearch = key.length >= 2;

  useEffect(() => {
    if (!active || !canSearch) return;
    const cached = cache.current.get(key);
    if (cached) { setResponse({ key, items: cached, error: null, loading: false }); return; }
    let live = true;
    setResponse({ key, items: [], error: null, loading: true });
    const timer = window.setTimeout(() => {
      desktop<HomebrewResult[]>("search_homebrew", { query: key })
        .then(items => {
          cache.current.set(key, items);
          if (live) setResponse({ key, items, error: null, loading: false });
        })
        .catch(error => {
          if (live) setResponse({ key, items: [], error: String(error), loading: false });
        });
    }, 250);
    return () => { live = false; window.clearTimeout(timer); };
  }, [active, key, canSearch, attempt]);

  const curated = useMemo(() => key ? filterCurated(SOFTWARE, query, "全部")
    .sort((a, b) => b.popularity - a.popularity) : [], [key, query]);
  const remote = useMemo(() => {
    if (!canSearch || response.key !== key) return [];
    const seen = new Set(curated.map(packageKey));
    return response.items.filter(item => {
      const id = packageKey(item);
      if (seen.has(id)) return false;
      seen.add(id);
      return true;
    }).map(remoteSoftware);
  }, [canSearch, key, response, curated]);

  return {
    curated, remote,
    searching: active && canSearch && (response.key !== key || response.loading),
    error: canSearch && response.key === key ? response.error : null,
    retry: () => { cache.current.delete(key); setAttempt(value => value + 1); }
  };
}
