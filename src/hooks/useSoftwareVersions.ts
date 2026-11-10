import { useEffect, useState } from "react";
import { desktop } from "../lib/ipc";
import type { VersionCatalog } from "../lib/ipc";

export function useSoftwareVersions(softwareId: string | null, refreshKey: string) {
  const [catalog, setCatalog] = useState<VersionCatalog | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    if (!softwareId) { setCatalog(null); setLoading(false); setError(null); return; }
    let live = true;
    setLoading(true);
    setError(null);
    desktop<VersionCatalog>("get_software_versions", { softwareId })
      .then(value => { if (live) { setCatalog(value); if (value.error) setError(value.error); } })
      .catch(value => { if (live) { setCatalog(null); setError(String(value)); } })
      .finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [softwareId, refreshKey, tick]);

  return { catalog, loading, error, retry: () => setTick(value => value + 1) };
}
