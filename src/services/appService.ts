import { call } from "./ipc";
import type { AppInfo } from "../types/app";

/** Round-trips to Rust, proving the IPC boundary works (R7.3). */
export function ping(): Promise<AppInfo> {
  return call<AppInfo>("ping");
}

/**
 * Tell the backend the window has painted, so startup time reaches the log.
 *
 * Fire-and-forget, and deliberately silent on failure: a measurement must
 * never be able to break the thing it is measuring.
 */
export function reportWindowReady(): void {
  void call<void>("report_window_ready").catch(() => {});
}

/**
 * The welcome note, if this launch just created one. Returns it once.
 *
 * Safe to call more than once and from more than one place — the backend
 * clears it as it hands it over.
 */
export function takeFirstRunNote(): Promise<string | null> {
  return call<string | null>("take_first_run_note");
}
