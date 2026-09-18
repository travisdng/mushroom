import { useEffect, useMemo, useState } from "react";

import { Dialog } from "../common/Dialog";
import {
  getPrivacyRules,
  setDisabledPrivacyRules,
} from "../../services/aiService";

/**
 * Which credential patterns Mushroom looks for.
 *
 * There are over two hundred, so the list is filtered rather than scrolled.
 * Switching one off is a real choice with a real consequence, which the dialog
 * says plainly rather than hiding behind a checkbox.
 */
export function PrivacyRulesDialog({
  disabled,
  onChange,
  onClose,
}: {
  disabled: string[];
  onChange: (disabled: string[]) => void;
  onClose: () => void;
}) {
  const [available, setAvailable] = useState<string[]>([]);
  const [off, setOff] = useState<string[]>(disabled);
  const [filter, setFilter] = useState("");

  useEffect(() => {
    let cancelled = false;
    getPrivacyRules()
      .then((rules) => {
        if (cancelled) return;
        setAvailable(rules.available);
        setOff(rules.disabled);
      })
      .catch(() => {
        // An empty list is honest here: it shows nothing rather than
        // pretending every rule is on.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const shown = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    if (!needle) return available;
    return available.filter((name) => name.includes(needle));
  }, [available, filter]);

  const toggle = (name: string) => {
    setOff((current) =>
      current.includes(name)
        ? current.filter((n) => n !== name)
        : [...current, name],
    );
  };

  const accept = async () => {
    try {
      await setDisabledPrivacyRules(off);
      onChange(off);
    } catch {
      // Leave the dialog open rather than claiming a save that did not happen.
      return;
    }
    onClose();
  };

  return (
    <Dialog
      title="Detection Rules"
      onClose={onClose}
      onAccept={() => void accept()}
      width={460}
    >
      <p className="settings__note">
        Ticked rules are used to find credentials before a question is sent.
        Switching one off means Mushroom stops looking for that kind.
      </p>

      <input
        type="search"
        className="field__input"
        placeholder="Filter rules"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        aria-label="Filter rules"
      />

      <div className="rule-list" role="group" aria-label="Detection rules">
        {shown.length === 0 ? (
          <div className="settings__note">No rule matches that.</div>
        ) : (
          shown.map((name) => (
            <label key={name} className="rule-list__row">
              <input
                type="checkbox"
                checked={!off.includes(name)}
                onChange={() => toggle(name)}
              />
              {name}
            </label>
          ))
        )}
      </div>

      <div className="settings__note">
        {available.length} rules · {off.length} switched off
      </div>
    </Dialog>
  );
}
