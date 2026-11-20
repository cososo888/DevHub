import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "../lib/ipc";
import type { InstallFinished, InstallProgress, PackageKind } from "../lib/ipc";

export function useInstallTask() {
  const busy = useRef(false);
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
          if (active) { busy.current = true; setTask(event.payload); setFailure(null); }
        });
        unlistenFinished = await listen<InstallFinished>("installation://finished", event => {
          if (active) { busy.current = false; setTask(null); setResult(event.payload); }
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
    if (busy.current) {
      setFailure({ message: "已有软件任务正在进行，请等它完成。", package: packageName, kind });
      return;
    }
    busy.current = true;
    setFailure(null);
    setResult(null);
    setTask({ jobId: 0, kind, softwareId: packageName, version: packageName, action, stage: "starting", message: action === "install" ? "正在提交安装…" : "正在提交卸载…", percent: null });
    try {
      // Ask in context after the user's install confirmation. Consent is optional;
      // denial or an unavailable notification service never prevents installation.
      if (action === "install") await desktop("request_notification_permission").catch(() => null);
      await desktop<number>(action === "install" ? "start_package_install" : "start_package_uninstall", { package: packageName, kind });
    } catch (value) {
      busy.current = false;
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
