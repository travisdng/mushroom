import { call } from "./ipc";
import type { AppConfig, UiState } from "../types/config";

export function getConfig(): Promise<AppConfig> {
  return call<AppConfig>("get_config");
}

export function setUiState(ui: UiState): Promise<void> {
  return call<void>("set_ui_state", { ui });
}
