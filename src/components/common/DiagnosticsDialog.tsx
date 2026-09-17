import { useCallback, useEffect, useState } from "react";

import { Button } from "./Button";
import { Dialog } from "./Dialog";
import * as diag from "../../services/diagnosticsService";
import { toAppError } from "../../services/ipc";
import { LOG_CATEGORIES, LOG_LEVELS } from "../../types/diagnostics";
import type { Diagnostics, LogLine } from "../../types/diagnostics";

/**
 * What Mushroom knows about itself.
 *
 * Three etched group boxes over a sunken log view, and a `Copy Diagnostics`
 * button whose whole point is that the result can be pasted anywhere without
 * reading it first — the backend redacts keys and paths on the way out.
 */
export function DiagnosticsDialog({
  onClose,
  onRebuildIndex,
  onOpenSettings,
}: {
  onClose: () => void;
  onRebuildIndex: () => void;
  onOpenSettings: () => void;
}) {
  const [info, setInfo] = useState<Diagnostics | null>(null);
  const [lines, setLines] = useState<LogLine[]>([]);
  const [category, setCategory] = useState("");
  const [level, setLevel] = useState("");
  const [note, setNote] = useState<string | null>(null);

  const refresh = useCallback(() => {
    diag
      .getDiagnostics()
      .then(setInfo)
      .catch((raw) => setNote(toAppError(raw).message));
    diag
      .getLogTail(category, level)
      .then(setLines)
      .catch(() => setLines([]));
  }, [category, level]);

  useEffect(refresh, [refresh]);

  const copy = useCallback(async () => {
    try {
      const report = await diag.copyDiagnostics();
      await navigator.clipboard.writeText(report);
      setNote("Copied. It contains no note content, paths or keys.");
    } catch (raw) {
      setNote(toAppError(raw).message);
    }
  }, []);

  return (
    <Dialog
      title="Diagnostics"
      onClose={onClose}
      acceptLabel="Close"
      acceptOnly
      width={620}
      footerExtra={
        <>
          <Button onClick={() => void copy()}>Copy Diagnostics</Button>
          <Button onClick={() => void diag.openLogFolder()}>Open Log Folder</Button>
        </>
      }
    >
      {info ? (
        <>
          <fieldset className="groupbox">
            <legend>Application</legend>
            <Facts
              rows={[
                ["Version", info.app.version],
                ["Platform", info.app.platform],
                ["Settings version", String(info.app.configVersion)],
                ["Notes folder", info.app.notesRoot ?? "(none)"],
                ["Notes", `${info.app.notesCount} (${info.app.notesSkipped} skipped)`],
                [
                  "Watching folder",
                  info.app.watching ? "yes" : "no — use F5 to refresh",
                ],
                ["Data folder", info.app.dataDir],
              ]}
            />
          </fieldset>

          <fieldset className="groupbox">
            <legend>Index</legend>
            <Facts
              rows={[
                ["Database", info.index.databasePath ?? "(not open)"],
                ["Notes indexed", String(info.index.noteCount)],
                ["Passages", String(info.index.passageCount)],
                ["Skipped", String(info.index.skipped)],
                [
                  "State",
                  info.index.indexing
                    ? "indexing now"
                    : info.index.stale
                      ? "out of date"
                      : "up to date",
                ],
                ["Last full rebuild", info.index.lastFullRebuild ?? "(never)"],
              ]}
            />
            <div className="diagnostics__actions">
              <Button onClick={onRebuildIndex}>Rebuild Index…</Button>
            </div>
          </fieldset>

          <fieldset className="groupbox">
            <legend>AI Service</legend>
            <Facts
              rows={[
                ["Configured", info.ai.configured ? "yes" : "no"],
                ["Provider", info.ai.provider],
                ["Endpoint", info.ai.endpoint],
                ["Model", info.ai.model],
                ["API key", info.ai.keyStatus],
                ["Streaming", info.ai.streaming ? "yes" : "no"],
                ["Last test", info.ai.lastConnection ?? "(not tested)"],
                [
                  "This session",
                  `${info.ai.requests} requests, ${info.ai.totalTokens.toLocaleString()} tokens, ${info.ai.failures} failed`,
                ],
              ]}
            />
            <div className="diagnostics__actions">
              <Button onClick={onOpenSettings}>Test Connection…</Button>
            </div>
          </fieldset>
        </>
      ) : (
        <p style={{ margin: 0 }}>Gathering…</p>
      )}

      <fieldset className="groupbox">
        <legend>Log</legend>
        <div className="diagnostics__filters">
          <label htmlFor="diag-category">Category:</label>
          <select
            id="diag-category"
            className="field"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          >
            <option value="">All</option>
            {LOG_CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>

          <label htmlFor="diag-level">Level:</label>
          <select
            id="diag-level"
            className="field"
            value={level}
            onChange={(e) => setLevel(e.target.value)}
          >
            <option value="">All</option>
            {LOG_LEVELS.map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
          </select>

          <Button onClick={refresh}>Refresh</Button>
          <Button
            onClick={() => {
              void diag.deleteLogs().then((n) => {
                setNote(`${n} old log ${n === 1 ? "file" : "files"} deleted.`);
                refresh();
              });
            }}
          >
            Delete Old Logs
          </Button>
        </div>

        <div className="diagnostics__log selectable">
          {lines.length === 0 ? (
            <div className="diagnostics__empty">Nothing logged yet.</div>
          ) : (
            lines.map((line, i) => (
              <div key={i} className="diagnostics__line" data-level={line.level}>
                <span className="diagnostics__time">
                  {line.timestamp.slice(11, 19)}
                </span>
                <span className="diagnostics__level">{line.level}</span>
                <span className="diagnostics__category">{line.category}</span>
                <span className="diagnostics__message">{line.message}</span>
              </div>
            ))
          )}
        </div>
      </fieldset>

      {note ? <div className="settings__apply-note">{note}</div> : null}
    </Dialog>
  );
}

function Facts({ rows }: { rows: Array<[string, string]> }) {
  return (
    <table className="diagnostics__facts">
      <tbody>
        {rows.map(([label, value]) => (
          <tr key={label}>
            <td className="diagnostics__label">{label}:</td>
            <td className="diagnostics__value selectable">{value}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
