import { call } from "./ipc";
import type { AppInfo } from "../types/app";

/** Round-trips to Rust, proving the IPC boundary works (R7.3). */
export function ping(): Promise<AppInfo> {
  return call<AppInfo>("ping");
}
