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
  /** How hard the privacy gate tries before anything is sent. */
  privacyMode: PrivacyMode;
  /** Folders and globs never sent to an AI endpoint. */
  aiExclusions: ExclusionRule[];
  /** Detection rules switched off, by name. */
  aiDisabledRules: string[];
  /** Whether AI settings have ever been applied. */
  configured: boolean;
};

/**
 * `redact` replaces what it recognises, `block` withholds the whole item,
 * `off` sends as typed. Exclusion applies in all three — that is the user's
 * own instruction, not something Mushroom inferred.
 */
export type PrivacyMode = "redact" | "block" | "off";

export type ExclusionRule = {
  pattern: string;
  /** Present when the rule could not be understood. */
  problem?: string;
};

/** Rule name and how many times it fired. Never the matched text. */
export type Redaction = { rule: string; count: number };

/** Something withheld whole, in `block` mode. */
export type Withheld = { kind: string; rule: string };

/** What the gate did to one request. */
export type PrivacyReport = {
  redactions: Redaction[];
  withheld: Withheld[];
  excludedNotes: number;
  mode: PrivacyMode;
};

/** The request as it actually went on the wire. Local only. */
export type LastRequest = {
  messages: { role: string; content: string }[];
  model: string;
  report: PrivacyReport;
  at: string;
};

export type PrivacyRules = {
  available: string[];
  disabled: string[];
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
