import { invoke } from "@tauri-apps/api/core";

export type ScanRoot = { id: number; path: string; enabled: boolean };
export type ScanStatus = "queued" | "running" | "paused" | "awaiting_confirmation" | "importing" | "completed" | "cancelled" | "failed";
export type ScanAction = "pause" | "resume" | "cancel" | "confirm";

export type ScanJob = {
  id: number;
  rootId: number;
  path: string;
  status: ScanStatus;
  startedAt: number | null;
  finishedAt: number | null;
  error: string | null;
  found: number;
  bytes: number;
  newFiles: number;
  existing: number;
  duplicates: number;
  invalid: number;
  inaccessible: number;
  directoryErrors: number;
  imported: number;
  failed: number;
  skipped: number;
};

export type ScanSnapshot = { roots: ScanRoot[]; jobs: ScanJob[] };

export const getScanSnapshot = (before: number | null) => invoke<ScanSnapshot>("scan_snapshot", { before });
export const pickScanRoots = () => invoke<boolean>("pick_scan_roots");
export const updateScanRoot = (id: number, enabled: boolean | null) => invoke<void>("update_scan_root", { id, enabled });
export const startScan = (id: number) => invoke<void>("start_scan", { id });
export const controlScan = (id: number, action: ScanAction) => invoke<void>("control_scan", { id, action });

export function scanError(error: unknown) {
  if (typeof error === "object" && error !== null && "message" in error && typeof error.message === "string") {
    return error.message;
  }
  return "LibraDesk could not complete the scan request. Try again.";
}
