import { useEffect, useState } from "react";

import { Button } from "../common/Button";
import { PrivacyRulesDialog } from "./PrivacyRulesDialog";
import { setAiExclusions } from "../../services/aiService";
import type { AiConfig, ExclusionRule, PrivacyMode } from "../../types/ai";

/**
 * What leaves this machine, and how hard Mushroom tries to stop it.
 *
 * Deliberately blunt about what it can and cannot promise: exclusion is
 * complete, detection is best-effort. Nothing here claims Mushroom removes
 * secrets, because a user who believes that is a user who stops marking their
 * notes — and marking them is the control that actually works.
 */
export function PrivacySettings({
  draft,
  update,
  endpoint,
}: {
  draft: AiConfig;
  update: (patch: Partial<AiConfig>) => void;
  endpoint: string;
}) {
  // Edited as text, one pattern per line: it is how people think about a list
  // of folders, and it beats a list box with Add/Remove for ten items.
  const [patterns, setPatterns] = useState(
    draft.aiExclusions.map((r) => r.pattern).join("\n"),
  );
  const [problems, setProblems] = useState<ExclusionRule[]>(
    draft.aiExclusions.filter((r) => r.problem),
  );
  const [showRules, setShowRules] = useState(false);

  useEffect(() => {
    setPatterns(draft.aiExclusions.map((r) => r.pattern).join("\n"));
  }, [draft.aiExclusions]);

  const saveExclusions = async () => {
    const lines = patterns
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    try {
      const parsed = await setAiExclusions(lines);
      setProblems(parsed.filter((r) => r.problem));
      update({ aiExclusions: parsed });
    } catch {
      // Saving settings is not worth an alert here; the next save retries.
    }
  };

  return (
    <fieldset className="groupbox">
      <legend>Privacy</legend>

      <div className="settings__note">
        Note content is sent to <strong>{endpoint || "the AI endpoint"}</strong>{" "}
        when you ask a question.
      </div>

      <div className="settings__row">
        <span className="settings__label">When sending:</span>
        <div className="radio-group" role="radiogroup" aria-label="Privacy mode">
          {(
            [
              ["redact", "Replace credentials I recognise"],
              ["block", "Leave out anything that looks like a credential"],
              ["off", "Send my notes as they are"],
            ] as [PrivacyMode, string][]
          ).map(([mode, label]) => (
            <label key={mode} className="radio">
              <input
                type="radio"
                name="privacy-mode"
                checked={draft.privacyMode === mode}
                onChange={() => {
                  if (mode === "off") {
                    const ok = window.confirm(
                      `Send your notes to ${endpoint} without checking them for passwords and keys?\n\n` +
                        `Notes you have marked with "ai: false" are still never sent.`,
                    );
                    if (!ok) return;
                  }
                  update({ privacyMode: mode });
                }}
              />
              {label}
            </label>
          ))}
        </div>
      </div>

      <div className="settings__note">
        Detection is best-effort and will miss things. To be certain a note is
        never sent, put <code>ai: false</code> in its frontmatter or exclude its
        folder below — that works in every mode.
      </div>

      <div className="settings__row settings__row--stacked">
        <span className="settings__label">Never send:</span>
        <textarea
          className="field__input exclusions"
          rows={4}
          spellCheck={false}
          value={patterns}
          onChange={(e) => setPatterns(e.target.value)}
          onBlur={() => void saveExclusions()}
          aria-label="Folders and patterns never sent to AI"
          placeholder={"personal\nwork/credentials/**\n**/secrets.md"}
        />
      </div>
      <div className="settings__note">
        One folder or pattern per line, relative to your notes folder.
        <code>**</code> matches any depth.
      </div>

      {problems.length > 0 ? (
        <div className="settings__problem" role="alert">
          {problems.map((rule) => (
            <div key={rule.pattern}>
              <strong>{rule.pattern || "(empty)"}</strong> — {rule.problem}
            </div>
          ))}
        </div>
      ) : null}

      <div className="settings__row">
        <span className="settings__label">Detection rules:</span>
        <Button onClick={() => setShowRules(true)}>Choose rules…</Button>
        {draft.aiDisabledRules.length > 0 ? (
          <span className="settings__inline-note">
            {draft.aiDisabledRules.length} switched off
          </span>
        ) : null}
      </div>

      {showRules ? (
        <PrivacyRulesDialog
          disabled={draft.aiDisabledRules}
          onChange={(disabled) => update({ aiDisabledRules: disabled })}
          onClose={() => setShowRules(false)}
        />
      ) : null}
    </fieldset>
  );
}
