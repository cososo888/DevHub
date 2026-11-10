import { useEffect, useState } from "react";
import { desktop } from "../lib/ipc";
import type { InstallPreview, UninstallPreview } from "../lib/ipc";

export function useHomebrewPreview(softwareId: string | null, refreshKey: number) {
  const [preview, setPreview] = useState<InstallPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!softwareId) { setPreview(null); setLoading(false); setError(null); return; }
    let live = true;
    setLoading(true);
    setError(null);
    desktop<InstallPreview>("preview_homebrew_install", { softwareId })
      .then(value => { if (live) setPreview(value); })
      .catch(value => { if (live) { setPreview(null); setError(String(value)); } })
      .finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [softwareId, refreshKey]);

  return { preview, loading, error };
}

export function useHomebrewUninstallPreview(softwareId: string | null, refreshKey: number) {
  const [preview, setPreview] = useState<UninstallPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!softwareId) { setPreview(null); setLoading(false); setError(null); return; }
    let live = true;
    setLoading(true);
    setError(null);
    desktop<UninstallPreview>("preview_homebrew_uninstall", { softwareId })
      .then(value => { if (live) setPreview(value); })
      .catch(value => { if (live) { setPreview(null); setError(String(value)); } })
      .finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [softwareId, refreshKey]);

  return { preview, loading, error };
}
