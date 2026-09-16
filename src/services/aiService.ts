import { call } from "./ipc";
import type {
  AiConfig,
  AiSettings,
  LastConnection,
  ModelList,
  Provider,
  ProviderDefaults,
} from "../types/ai";

export function getAiSettings(): Promise<AiSettings> {
  return call<AiSettings>("get_ai_settings");
}

export function getProviderDefaults(
  provider: Provider,
): Promise<ProviderDefaults> {
  return call<ProviderDefaults>("get_provider_defaults", { provider });
}

export function setAiConfig(config: AiConfig): Promise<AiSettings> {
  return call<AiSettings>("set_ai_config", { config });
}

/**
 * The key travels in this direction only. Nothing in this module can read it
 * back — the backend returns a status instead.
 */
export function setAiKey(
  provider: Provider,
  key: string,
): Promise<AiSettings["keyStatus"]> {
  return call<AiSettings["keyStatus"]>("set_ai_key", {
    input: { provider, key },
  });
}

export function clearAiKey(
  provider: Provider,
): Promise<AiSettings["keyStatus"]> {
  return call<AiSettings["keyStatus"]>("clear_ai_key", { provider });
}

/** The status for a provider that may not be the applied one yet. */
export function getKeyStatus(
  provider: Provider,
): Promise<AiSettings["keyStatus"]> {
  return call<AiSettings["keyStatus"]>("get_key_status", { provider });
}

/** Tests `config` if given, otherwise whatever is saved. */
export function testAiConnection(config?: AiConfig): Promise<LastConnection> {
  return call<LastConnection>("test_ai_connection", { config: config ?? null });
}

/** Lists models from `config` if given, otherwise from whatever is saved. */
export function listAiModels(config?: AiConfig): Promise<ModelList> {
  return call<ModelList>("list_ai_models", { config: config ?? null });
}
