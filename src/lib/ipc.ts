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
