import { useState } from "react";
import { Button } from "../components/common/Button";
import { Field, TextArea } from "../components/common/Field";
import { Panel } from "../components/common/Panel";
import { Dialog } from "../components/common/Dialog";
import { EmptyState } from "../components/common/EmptyState";
import { Icon } from "../components/common/Icon";
import { AiUnconfigured } from "../components/common/AiUnconfigured";

/**
 * Temporary control gallery, reachable at #gallery in `npm run dev`.
 *
 * Exists so every control state can be seen side by side and checked against
 * the banned list in .kiro/steering/ui-retro.md. Removed before v1.0.
 */
export default function Gallery() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [text, setText] = useState("GPU failure");

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 12 }}>
      <h1 style={{ fontSize: 13, margin: "0 0 12px" }}>
        Mushroom control gallery
      </h1>

      <fieldset className="groupbox">
        <legend>Buttons</legend>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <Button>Default</Button>
          <Button data-pressed="true" pressed>
            Pressed
          </Button>
          <Button disabled>Disabled</Button>
          <Button>
            <Icon name="mushroom" /> With icon
          </Button>
        </div>
      </fieldset>

      <fieldset className="groupbox">
        <legend>Toolbar buttons</legend>
        <div
          style={{
            display: "flex",
            gap: 2,
            height: 38,
            alignItems: "stretch",
            background: "var(--surface)",
          }}
        >
          {["New", "Open", "Save", "Search", "AI"].map((label) => (
            <Button key={label} variant="toolbar" disabled={label !== "New"}>
              <Icon name="mushroom" />
              {label}
            </Button>
          ))}
        </div>
      </fieldset>

      <fieldset className="groupbox">
        <legend>Fields</legend>
        <Field
          label="Endpoint"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <Field label="Model" placeholder="gpt-4.1-mini" />
        <Field label="Disabled" value="cannot edit" disabled readOnly />
        <Field
          label="Invalid"
          defaultValue="not-a-url"
          error="Enter a URL beginning with http:// or https://"
        />
        <div style={{ marginTop: 6 }}>
          <TextArea rows={3} defaultValue={"# AI AutoQA\n\nThe orchestrator..."} />
        </div>
      </fieldset>

      <fieldset className="groupbox">
        <legend>Panels</legend>
        <div style={{ display: "flex", gap: 8, height: 160 }}>
          <Panel title="Notebook" style={{ width: 200 }}>
            <div style={{ padding: 2 }}>
              {["Work", "Projects", "Ideas", "Personal"].map((f, i) => (
                <div
                  key={f}
                  style={{
                    height: "var(--row-height)",
                    lineHeight: "var(--row-height)",
                    padding: "0 4px",
                    background: i === 1 ? "var(--selection-bg)" : undefined,
                    color: i === 1 ? "var(--selection-text)" : undefined,
                  }}
                >
                  {f}
                </div>
              ))}
            </div>
          </Panel>

          <Panel title="Notes" style={{ width: 200 }}>
            <EmptyState text="No notes yet." />
          </Panel>

          <Panel title="AI Search" flat style={{ width: 240 }}>
            <div style={{ padding: 4 }}>
              <TextArea rows={3} placeholder="What did I write about…" />
              <div style={{ marginTop: 4 }}>
                <Button disabled>Ask Mushroom</Button>
              </div>
            </div>
          </Panel>

          {/* Deliberately not styled as an error: nothing has gone wrong. */}
          <Panel title="AI Search" flat style={{ width: 260 }}>
            <AiUnconfigured onOpenSettings={() => setDialogOpen(true)} />
          </Panel>
        </div>
      </fieldset>

      <fieldset className="groupbox">
        <legend>Dialog</legend>
        <Button onClick={() => setDialogOpen(true)}>Open dialog…</Button>
        {dialogOpen ? (
          <Dialog
            title="About Mushroom"
            onClose={() => setDialogOpen(false)}
            acceptOnly
          >
            <div style={{ display: "flex", gap: 10 }}>
              <Icon name="mushroom" size={32} />
              <div>
                <div style={{ fontWeight: "bold" }}>Mushroom</div>
                <div>Personal knowledge and notes.</div>
                <div style={{ marginTop: 6 }}>Version 0.1.0</div>
              </div>
            </div>
          </Dialog>
        ) : null}
      </fieldset>

      <fieldset className="groupbox">
        <legend>Icon scaling</legend>
        <div style={{ display: "flex", gap: 10, alignItems: "flex-end" }}>
          {[16, 24, 32, 48].map((s) => (
            <Icon key={s} name="mushroom" size={s} title={`${s}px`} />
          ))}
        </div>
      </fieldset>
    </div>
  );
}
