import { invoke } from "@tauri-apps/api/core";

export interface AppStatus {
  phase: number;
  product: string;
  detection: string;
}

/** Thin IPC boundary: only fixed command names and IDs, never URLs or paths. */
export function desktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}
