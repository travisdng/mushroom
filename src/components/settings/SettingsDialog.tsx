import { useCallback, useEffect, useRef, useState } from "react";
import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";

import { Button } from "../common/Button";
import { Dialog } from "../common/Dialog";
import { Field } from "../common/Field";
import * as ai from "../../services/aiService";
import { setNotesRoot } from "../../services/notesService";
import { getConfig } from "../../services/configService";
import { toAppError } from "../../services/ipc";
import { PROVIDER_LABELS } from "../../types/ai";
import type { AiConfig, AiSettings, LastConnection, Provider } from "../../types/ai";

/** The defaults shown as "(default: …)" next to the Advanced fields. */
const DEFAULTS = {
  timeoutSecs: 120,
  maxContextTokens: 6000,
  temperature: 0.2,
};

type Draft = AiConfig & { notesRoot: string };

type Errors = Partial<Record<keyof Draft, string>>;

/**
 * Validated here as well as in Rust. The backend is the authority — this copy
 * exists so the message appears beside the field as you type rather than after
 * a round trip (R3.5).
 */
function validate(draft: Draft): Errors {
  const errors: Errors = {};

  const url = draft.baseUrl.trim();
  if (!url) errors.baseUrl = "Enter an endpoint URL.";
  else if (!/^https?:\/\//i.test(url))
    errors.baseUrl = "The endpoint must begin with http:// or https://";

  if (!draft.model.trim()) errors.model = "Enter a model name.";

  if (!Number.isFinite(draft.timeoutSecs) || draft.timeoutSecs < 1 || draft.timeoutSecs > 3600)
    errors.timeoutSecs = "Between 1 and 3600 seconds.";

  if (!Number.isFinite(draft.maxContextTokens) || draft.maxContextTokens < 500)
    errors.maxContextTokens = "At least 500 tokens.";

  if (!Number.isFinite(draft.temperature) || draft.temperature < 0 || draft.temperature > 2)
    errors.temperature = "Between 0 and 2.";

  if (!draft.notesRoot.trim()) errors.notesRoot = "Choose a notes folder.";

  return errors;
}

function keyStatusText(status: AiSettings["keyStatus"]): string {
  switch (status) {
    case "set":
      return "A key is stored in Windows Credential Manager.";
    case "sessionOnly":
      return "Stored for this session only — Credential Manager was unavailable.";
    default:
      return "No key stored.";
  }
}

export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [savedRoot, setSavedRoot] = useState("");
  const [loadError, setLoadError] = useState<string | null>(null);

  const [keyInput, setKeyInput] = useState<string | null>(null);
  const [keyStatus, setKeyStatus] = useState<AiSettings["keyStatus"]>("notSet");

  const [models, setModels] = useState<string[] | null>(null);
  const [modelsNote, setModelsNote] = useState<string | null>(null);
  const [modelsBusy, setModelsBusy] = useState(false);

  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<LastConnection | null>(null);
  const [applyNote, setApplyNote] = useState<string | null>(null);

  /**
   * Whether the user has typed over the endpoint or model. Switching provider
   * fills in defaults only while these are false — a hand-typed endpoint is
   * never silently overwritten (R1.4).
   */
  const touched = useRef({ baseUrl: false, model: false });

  useEffect(() => {
    let live = true;
    Promise.all([ai.getAiSettings(), getConfig()])
      .then(([aiSettings, config]) => {
        if (!live) return;
        const root = config.notesRoot ?? "";
        setSettings(aiSettings);
        setKeyStatus(aiSettings.keyStatus);
        // Only if it still describes the endpoint in the fields. A result left
        // over from an endpoint that was tried and discarded reads as a verdict
        // on the current one.
        setTestResult(
          aiSettings.lastConnection?.endpoint === aiSettings.config.baseUrl
            ? aiSettings.lastConnection
            : null,
        );
        setSavedRoot(root);
        setDraft({ ...aiSettings.config, notesRoot: root });
      })
      .catch((raw) => {
        if (live) setLoadError(toAppError(raw).message);
      });
    return () => {
      live = false;
    };
  }, []);

  const update = useCallback((patch: Partial<Draft>) => {
    setApplyNote(null);
    // A result for the old endpoint or model would be read as a result for the
    // new one, which is worse than showing nothing.
    if ("baseUrl" in patch || "model" in patch) setTestResult(null);
    // Models came from the old endpoint and may not exist on the new one.
    if ("baseUrl" in patch) {
      setModels(null);
      setModelsNote(null);
    }
    setDraft((current) => (current ? { ...current, ...patch } : current));
  }, []);

  const onProviderChange = useCallback(
    async (provider: Provider) => {
      update({ provider });
      try {
        const defaults = await ai.getProviderDefaults(provider);
        // Only fill in what the user has not made their own.
        const patch: Partial<Draft> = { provider };
        if (!touched.current.baseUrl) patch.baseUrl = defaults.baseUrl;
        if (!touched.current.model) patch.model = defaults.model;
        update(patch);
      } catch {
        // Defaults are a convenience; failing to fetch them is not an error
        // worth a dialog, and the fields keep their current values.
      }
      // The key is stored per provider, so ask about the one now selected —
      // not the one last applied, which is what the saved settings describe.
      try {
        setKeyStatus(await ai.getKeyStatus(provider));
      } catch {
        /* leave the last known status */
      }
      setModels(null);
      setModelsNote(null);
      // A result from the previous endpoint says nothing about this one.
      setTestResult(null);
    },
    [update],
  );

  /** The settings as typed, for the calls that must not use the saved ones. */
  const candidate = useCallback((): AiConfig | undefined => {
    if (!draft) return undefined;
    const { notesRoot: _root, ...rest } = draft;
    return { ...rest, baseUrl: rest.baseUrl.trim(), model: rest.model.trim() };
  }, [draft]);

  const refreshModels = useCallback(async () => {
    setModelsBusy(true);
    setModelsNote(null);
    try {
      const list = await ai.listAiModels(candidate());
      setModels(list.models);
      setModelsNote(list.message);
    } catch (raw) {
      setModels([]);
      setModelsNote(toAppError(raw).message);
    } finally {
      setModelsBusy(false);
    }
  }, [candidate]);

  const browse = useCallback(async () => {
    const chosen = await openFolderPicker({
      directory: true,
      multiple: false,
      title: "Choose the notes folder",
      defaultPath: draft?.notesRoot || undefined,
    });
    if (typeof chosen === "string") update({ notesRoot: chosen });
  }, [draft?.notesRoot, update]);

  const errors = draft ? validate(draft) : {};
  const invalid = Object.keys(errors).length > 0;

  /** Persist everything. Returns false if anything was rejected. */
  const apply = useCallback(async (): Promise<boolean> => {
    if (!draft || Object.keys(validate(draft)).length > 0) return false;

    const { notesRoot, ...aiConfig } = draft;
    try {
      const next = await ai.setAiConfig({
        ...aiConfig,
        baseUrl: aiConfig.baseUrl.trim(),
        model: aiConfig.model.trim(),
      });
      setSettings(next);
      setKeyStatus(next.keyStatus);

      // Changing the notes folder re-scans and re-indexes, so only do it when
      // it actually changed (R3.4).
      if (notesRoot.trim() && notesRoot.trim() !== savedRoot) {
        const count = await setNotesRoot(notesRoot.trim());
        setSavedRoot(notesRoot.trim());
        setApplyNote(`Saved. Re-scanned ${count} notes in the new folder.`);
      } else {
        setApplyNote("Saved.");
      }
      return true;
    } catch (raw) {
      setApplyNote(toAppError(raw).message);
      return false;
    }
  }, [draft, savedRoot]);

  const onAccept = useCallback(() => {
    void apply().then((ok) => {
      if (ok) onClose();
    });
  }, [apply, onClose]);

  const test = useCallback(async () => {
    if (!draft) return;
    setTesting(true);
    try {
      // Test what is on screen, not what was last saved — and without saving
      // it, so Cancel still discards a wrong endpoint you only tried.
      setTestResult(await ai.testAiConnection(candidate()));
    } catch (raw) {
      setTestResult({
        ok: false,
        endpoint: draft?.baseUrl ?? "",
        model: draft?.model ?? "",
        latencyMs: null,
        message: toAppError(raw).message,
        at: new Date().toISOString(),
      });
    } finally {
      setTesting(false);
    }
  }, [candidate, draft]);

  if (loadError) {
    return (
      <Dialog title="Settings" onClose={onClose} acceptOnly width={420}>
        <p style={{ margin: 0 }}>{loadError}</p>
      </Dialog>
    );
  }

  if (!draft || !settings) {
    return (
      <Dialog title="Settings" onClose={onClose} acceptOnly width={420}>
        <p style={{ margin: 0 }}>Loading settings…</p>
      </Dialog>
    );
  }

  const rootChanged = draft.notesRoot.trim() !== savedRoot;

  return (
    <Dialog
      title="Settings"
      onClose={onClose}
      onAccept={onAccept}
      acceptDisabled={invalid}
      width={520}
      footerExtra={
        <Button onClick={() => void apply()} disabled={invalid}>
          Apply
        </Button>
      }
    >
      <fieldset className="groupbox">
        <legend>AI Service</legend>

        <div className="field-row">
          <label htmlFor="settings-provider">Provider:</label>
          <select
            id="settings-provider"
            className="field"
            value={draft.provider}
            onChange={(e) => void onProviderChange(e.target.value as Provider)}
          >
            {(Object.keys(PROVIDER_LABELS) as Provider[]).map((p) => (
              <option key={p} value={p}>
                {PROVIDER_LABELS[p]}
              </option>
            ))}
          </select>
        </div>

        <Field
          label="Endpoint:"
          value={draft.baseUrl}
          error={errors.baseUrl}
          spellCheck={false}
          onChange={(e) => {
            touched.current.baseUrl = true;
            update({ baseUrl: e.target.value });
          }}
        />

        <div className="field-row">
          <label htmlFor="settings-model">Model:</label>
          <input
            id="settings-model"
            className="field"
            list="settings-model-options"
            value={draft.model}
            spellCheck={false}
            aria-invalid={errors.model ? true : undefined}
            onChange={(e) => {
              touched.current.model = true;
              update({ model: e.target.value });
            }}
          />
          <datalist id="settings-model-options">
            {(models ?? []).map((m) => (
              <option key={m} value={m} />
            ))}
          </datalist>
          <Button onClick={() => void refreshModels()} disabled={modelsBusy}>
            {modelsBusy ? "…" : "Refresh"}
          </Button>
        </div>
        {errors.model ? <div className="field-error">{errors.model}</div> : null}

        {models && models.length > 0 ? (
          // A plain list, not only the field's own datalist. A datalist
          // filters its options by what is already typed, so clicking the
          // arrow with a model name in the box shows you that one name — which
          // is no use at all when the point of Refresh is to find out what the
          // service offers.
          <div className="field-row">
            <span style={{ minWidth: 90 }} />
            <select
              className="field"
              value=""
              aria-label="Models this service offers"
              onChange={(e) => {
                if (!e.target.value) return;
                touched.current.model = true;
                update({ model: e.target.value });
              }}
            >
              <option value="">
                {models.length} model{models.length === 1 ? "" : "s"} offered…
              </option>
              {models.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </div>
        ) : null}

        {modelsNote ? <div className="settings__note">{modelsNote}</div> : null}

        <div className="field-row">
          <label htmlFor="settings-key">API key:</label>
          {keyInput === null ? (
            <input
              id="settings-key"
              className="field"
              type="password"
              value={keyStatus === "notSet" ? "" : "••••••••••••"}
              readOnly
              disabled
            />
          ) : (
            <input
              id="settings-key"
              className="field"
              type="password"
              value={keyInput}
              autoFocus
              spellCheck={false}
              placeholder="Paste the key"
              onChange={(e) => setKeyInput(e.target.value)}
            />
          )}
          {keyInput === null ? (
            <Button onClick={() => setKeyInput("")}>Change…</Button>
          ) : (
            <Button
              onClick={() => {
                void ai
                  .setAiKey(draft.provider, keyInput)
                  .then(setKeyStatus)
                  .then(() => setKeyInput(null))
                  .catch((raw) => setApplyNote(toAppError(raw).message));
              }}
              disabled={!keyInput.trim()}
            >
              Save key
            </Button>
          )}
          <Button
            onClick={() => {
              setKeyInput(null);
              void ai.clearAiKey(draft.provider).then(setKeyStatus);
            }}
            disabled={keyInput === null && keyStatus === "notSet"}
          >
            {keyInput === null ? "Clear" : "Cancel"}
          </Button>
        </div>
        <div className="settings__note">
          {keyStatusText(keyStatus)}
          {settings.keyRequired && keyStatus === "notSet"
            ? " This provider requires one."
            : ""}
        </div>

        <div className="field-row" style={{ marginTop: 8 }}>
          <span style={{ minWidth: 90 }} />
          <Button onClick={() => void test()} disabled={testing || invalid}>
            {testing ? "Testing…" : "Test Connection"}
          </Button>
          {testResult ? (
            <span
              className="settings__result"
              data-ok={testResult.ok ? "true" : "false"}
            >
              {testResult.message}
            </span>
          ) : null}
        </div>
      </fieldset>

      <fieldset className="groupbox">
        <legend>Notes</legend>

        <div className="field-row">
          <label htmlFor="settings-root">Folder:</label>
          <input
            id="settings-root"
            className="field"
            value={draft.notesRoot}
            spellCheck={false}
            aria-invalid={errors.notesRoot ? true : undefined}
            onChange={(e) => update({ notesRoot: e.target.value })}
          />
          <Button onClick={() => void browse()}>Browse…</Button>
        </div>
        {errors.notesRoot ? (
          <div className="field-error">{errors.notesRoot}</div>
        ) : null}
        {rootChanged ? (
          <div className="settings__note">
            Changing the folder re-scans it and rebuilds the search index. Your
            notes are not moved or modified.
          </div>
        ) : null}
      </fieldset>

      <fieldset className="groupbox">
        <legend>Advanced</legend>

        <Field
          label="Timeout:"
          type="number"
          min={1}
          max={3600}
          value={draft.timeoutSecs}
          error={errors.timeoutSecs}
          onChange={(e) => update({ timeoutSecs: Number(e.target.value) })}
        />
        <div className="settings__note">
          Seconds to wait for a reply. Default {DEFAULTS.timeoutSecs}.
        </div>

        <Field
          label="Context budget:"
          type="number"
          min={500}
          step={500}
          value={draft.maxContextTokens}
          error={errors.maxContextTokens}
          onChange={(e) => update({ maxContextTokens: Number(e.target.value) })}
        />
        <div className="settings__note">
          Tokens of notes sent with a question. Default{" "}
          {DEFAULTS.maxContextTokens}.
        </div>

        <Field
          label="Temperature:"
          type="number"
          min={0}
          max={2}
          step={0.1}
          value={draft.temperature}
          error={errors.temperature}
          onChange={(e) => update({ temperature: Number(e.target.value) })}
        />
        <div className="settings__note">
          Lower is more literal. Default {DEFAULTS.temperature}.
        </div>

        <div className="field-row">
          <span style={{ minWidth: 90 }} />
          <label className="settings__check">
            <input
              type="checkbox"
              checked={draft.logPrompts}
              onChange={(e) => update({ logPrompts: e.target.checked })}
            />
            Write prompts to the log file
          </label>
        </div>
        <div className="settings__note">
          Off by default. Prompts contain your note text, so only turn this on
          while diagnosing a problem.
        </div>
      </fieldset>

      {applyNote ? <div className="settings__apply-note">{applyNote}</div> : null}
    </Dialog>
  );
}
