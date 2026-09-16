import type { CSSProperties, ReactNode } from "react";

type PanelProps = {
  /** Section header text. Rendered uppercase, as NOTEBOOK / NOTES / AI SEARCH. */
  title: string;
  children: ReactNode;
  /** Drops the sunken white body, for panels holding controls rather than a list. */
  flat?: boolean;
  /** Extra chrome rendered at the right of the header bar. */
  headerExtra?: ReactNode;
  style?: CSSProperties;
};

export function Panel({ title, children, flat, headerExtra, style }: PanelProps) {
  return (
    <section className="panel" style={style} aria-label={title}>
      <div className="panel__header">
        {headerExtra ? (
          <span style={{ display: "flex", alignItems: "center" }}>
            <span style={{ flex: "1 1 auto" }}>{title.toUpperCase()}</span>
            {headerExtra}
          </span>
        ) : (
          title.toUpperCase()
        )}
      </div>
      <div className={flat ? "panel__body panel__body--flat" : "panel__body"}>
        {children}
      </div>
    </section>
  );
}
