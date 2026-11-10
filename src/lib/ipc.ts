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

export interface OwnershipView {
  id: number;
  status: string;
  preexisting: boolean;
  installedAt: number;
  instancePath: string | null;
  version: string | null;
}

export interface InstallPreview {
  softwareId: string;
  package: string | null;
  packageType: string | null;
  brewAvailable: boolean;
  brewVersion: string | null;
  prefix: string | null;
  formulaVersion: string | null;
  channel: string;
  alreadyInstalled: boolean;
  canInstall: boolean;
  reason: string | null;
  dependencies: string[];
  newDependencies: string[];
  installedDependencies: string[];
  ownership: OwnershipView | null;
}

export interface UninstallPreview {
  softwareId: string;
  package: string | null;
  version: string | null;
  instancePath: string | null;
  canUninstall: boolean;
  reason: string | null;
  dependents: string[];
  ownership: OwnershipView | null;
  warnings: string[];
}

export interface InstallProgress {
  jobId: number;
  softwareId: string;
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
  action: "install" | "uninstall";
  success: boolean;
  message: string;
  output: string;
  verification: Verification | null;
}
