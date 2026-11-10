import { invoke } from "@tauri-apps/api/core";

export interface AppStatus {
  phase: number;
  product: string;
  detection: string;
}

export type Health = "ok" | "unverified" | "broken";

export interface SoftwareInstance {
  softwareId: string;
  version: string | null;
  source: string;
  sources: string[];
  path: string;
  kind: string;
  executable: boolean;
  managedBy: string;
  health: Health;
  detail: string | null;
  lastDetectedAt: number;
}

export interface HomebrewStatus {
  available: boolean;
  prefix: string | null;
  version: string | null;
  error: string | null;
}

export interface DetectionError {
  source: string;
  message: string;
}

export interface DetectionSnapshot {
  schemaVersion: number;
  refreshedAt: number | null;
  stale: boolean;
  durationMs: number;
  homebrew: HomebrewStatus;
  instances: SoftwareInstance[];
  errors: DetectionError[];
}

/** Thin IPC boundary: fixed commands and IDs only, never URLs or paths. */
export function desktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export interface VersionInstance {
  source: string;
  path: string;
  health: string;
  managedBy: string;
  canUninstall: boolean;
  mechanism: string | null;
  reason: string | null;
  rootPath: string | null;
}

export interface VersionRow {
  version: string;
  installed: boolean;
  installable: boolean;
  canUninstall: boolean;
  methods: string[];
  formula: string | null;
  instances: VersionInstance[];
  reason: string | null;
  uninstallReason: string | null;
}

export interface VersionCatalog {
  softwareId: string;
  versions: VersionRow[];
  error: string | null;
  homebrewAvailable: boolean;
}

export interface InstallProgress {
  jobId: number;
  softwareId: string;
  version: string;
  action: "install" | "uninstall";
  stage: string;
  message: string;
  percent: number | null;
}

export interface Verification {
  verified: boolean;
  instancePath: string | null;
  version: string | null;
  health: string | null;
  systemGitAvailable: boolean | null;
}

export interface InstallFinished {
  jobId: number;
  softwareId: string;
  version: string;
  action: "install" | "uninstall";
  success: boolean;
  message: string;
  output: string;
  verification: Verification | null;
}
