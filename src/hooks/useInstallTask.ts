import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "../lib/ipc";
import type { InstallFinished, InstallProgress } from "../lib/ipc";

export function useInstallTask() {
  const [task, setTask] = useState<InstallProgress | null>(null);
  const [result, setResult] = useState<InstallFinished | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let unlistenProgress: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;
    const setup = async () => {
      try {
        unlistenProgress = await listen<InstallProgress>("installation://progress", event => {
          if (active) { setTask(event.payload); setError(null); }
        });
        unlistenFinished = await listen<InstallFinished>("installation://finished", event => {
          if (active) { setTask(null); setResult(event.payload); }
        });
      } catch {
        // Browser preview has no Tauri event bridge.
      }
    };
    void setup();
    return () => {
      active = false;
      unlistenProgress?.();
      unlistenFinished?.();
    };
  }, []);

  const start = useCallback(async (action: "install" | "uninstall", softwareId: string, version: string, formula?: string | null) => {
    setError(null);
    setResult(null);
    setTask({ jobId: 0, softwareId, version, action, stage: "starting", message: action === "install" ? "正在提交安装…" : "正在提交卸载…", percent: null });
    try {
      await desktop<number>(action === "install" ? "start_homebrew_install" : "start_homebrew_uninstall", action === "install" ? { softwareId, version } : { softwareId, version, formula: formula ?? null });
    } catch (value) {
      setTask(null);
      setError(String(value));
    }
  }, []);

  return {
    task,
    result,
    error,
    install: (softwareId: string, version: string) => start("install", softwareId, version),
    uninstall: (softwareId: string, version: string, formula?: string | null) => start("uninstall", softwareId, version, formula),
    clearResult: () => setResult(null)
  };
}
