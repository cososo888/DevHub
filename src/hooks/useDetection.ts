import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "../lib/ipc";
import type { DetectionSnapshot } from "../lib/ipc";

export function useDetection() {
  const [snapshot, setSnapshot] = useState<DetectionSnapshot | null>(null);
  const [refreshing, setRefreshing] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    setError(null);
    try {
      await desktop<boolean>("refresh_detection");
    } catch {
      setRefreshing(false);
      setError("检测仅在 DevHub V2 桌面应用中可用");
    }
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      try {
        unlisten = await listen<DetectionSnapshot>("detection://updated", event => {
          if (!active) return;
          setSnapshot(event.payload);
          setRefreshing(false);
          setError(null);
        });
      } catch {
        // Browser preview has no Tauri event bridge.
      }
      try {
        const cached = await desktop<DetectionSnapshot | null>("get_detection_snapshot");
        if (active && cached) setSnapshot(cached);
      } catch {
        // The refresh below will surface a useful error if this is not a desktop app.
      }
      if (active) await refresh();
    };
    void setup();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [refresh]);

  return { snapshot, refreshing, error, refresh };
}
