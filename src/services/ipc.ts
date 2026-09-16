import { invoke } from "@tauri-apps/api/core";
import type { AppErrorDto } from "../types/error";

/**
 * The only module that calls `invoke`. Everything else goes through a service
 * built on `call`, so no component ever touches Tauri directly.
 */

function isAppErrorDto(value: unknown): value is AppErrorDto {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.code === "string" &&
    typeof v.title === "string" &&
    typeof v.message === "string"
  );
}

/**
 * Normalise anything a rejected command throws into an AppErrorDto, so callers
 * never have to deal with a bare string or an unknown (R7.4).
 */
export function toAppError(raw: unknown): AppErrorDto {
  if (isAppErrorDto(raw)) return raw;

  // A panic, a serialisation failure, or the IPC channel itself failing.
  const detail =
    raw instanceof Error
      ? `${raw.name}: ${raw.message}`
      : typeof raw === "string"
        ? raw
        : JSON.stringify(raw);

  return {
    code: "IPC_FAILED",
    title: "Something went wrong inside Mushroom",
    message:
      "An unexpected problem stopped that action. Your notes are not affected.",
    detail,
    hint: "See Tools → Diagnostics for the technical details.",
  };
}

export async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    throw toAppError(raw);
  }
}

/** True when running inside the Tauri shell rather than a plain browser. */
export function inDesktopApp(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
