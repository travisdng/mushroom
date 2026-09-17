import { call } from "./ipc";
import type { Diagnostics, LogLine } from "../types/diagnostics";

export function getDiagnostics(): Promise<Diagnostics> {
  return call<Diagnostics>("get_diagnostics");
}

/** The report, already redacted — safe to paste anywhere. */
export function copyDiagnostics(): Promise<string> {
  return call<string>("copy_diagnostics");
}

export function getLogTail(
  category?: string,
  level?: string,
): Promise<LogLine[]> {
  return call<LogLine[]>("get_log_tail", {
    category: category ?? null,
    level: level ?? null,
  });
}

export function openLogFolder(): Promise<void> {
  return call<void>("open_log_folder");
}

export function deleteLogs(): Promise<number> {
  return call<number>("delete_logs");
}
