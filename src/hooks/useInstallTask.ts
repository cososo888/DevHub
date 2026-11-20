import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "../lib/ipc";
import type { InstallFinished, InstallProgress, PackageKind } from "../lib/ipc";

export function useInstallTask() {
  const [task, setTask] = useState<InstallProgress | null>(null);
  const [result, setResult] = useState<InstallFinished | null>(null);
  const [failure, setFailure] = useState<{ message: string; package: string; kind: PackageKind } | null>(null);

  useEffect(() => {
    let active = true;
    let unlistenProgress: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;
    const setup = async () => {
      try {
        unlistenProgress = await listen<InstallProgress>("installation://progress", event => {
          if (active) { setTask(event.payload); setFailure(null); }
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

  const start = useCallback(async (action: "install" | "uninstall", packageName: string, kind: PackageKind) => {
    setFailure(null);
    setResult(null);
    setTask({ jobId: 0, softwareId: packageName, version: packageName, action, stage: "starting", message: action === "install" ? "正在提交安装…" : "正在提交卸载…", percent: null });
    try {
      await desktop<number>(action === "install" ? "start_package_install" : "start_package_uninstall", { package: packageName, kind });
    } catch (value) {
      setTask(null);
      setFailure({ message: String(value), package: packageName, kind });
    }
  }, []);

  return {
    task,
    result,
    error: failure?.message ?? null,
    errorPackage: failure?.package ?? null,
    errorKind: failure?.kind ?? null,
    install: (packageName: string, kind: PackageKind) => start("install", packageName, kind),
    uninstall: (packageName: string, kind: PackageKind) => start("uninstall", packageName, kind)
  };
}
