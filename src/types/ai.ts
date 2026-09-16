/** Mirrors the AI types in src-tauri/src/ai/ and src-tauri/src/config/. */

/** Serde renames the Rust variants to camelCase. */
export type Provider = "liteLlm" | "openAi";

export const PROVIDER_LABELS: Record<Provider, string> = {
  liteLlm: "LiteLLM (proxy)",
  openAi: "OpenAI (direct)",
};

/** Whether a key is stored — never the key itself. */
export type KeyStatus = "set" | "notSet" | "sessionOnly";

export type AiConfig = {
  provider: Provider;
  baseUrl: string;
  model: string;
  timeoutSecs: number;
  maxContextTokens: number;
  temperature: number;
  logPrompts: boolean;
  stream: boolean;
  /** Whether AI settings have ever been applied. */
  configured: boolean;
};

export type LastConnection = {
  ok: boolean;
  endpoint: string;
  model: string;
  latencyMs: number | null;
  message: string;
  at: string;
};

export type AiSettings = {
  config: AiConfig;
  keyStatus: KeyStatus;
  keyRequired: boolean;
  defaultBaseUrl: string;
  defaultModel: string;
  lastConnection: LastConnection | null;
};

export type ProviderDefaults = {
  baseUrl: string;
  model: string;
  keyRequired: boolean;
};

export type ModelList = {
  models: string[];
  available: boolean;
  message: string | null;
};
