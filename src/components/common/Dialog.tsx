import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { Button } from "./Button";
import { Icon } from "./Icon";

type DialogProps = {
  title: string;
  children: ReactNode;
  /** Escape, the close box, and Cancel all route here. */
  onClose: () => void;
  /** Omit to render a single OK button that just closes. */
  onAccept?: () => void;
  acceptLabel?: string;
  cancelLabel?: string;
  /** Hides Cancel, for purely informational dialogs like About. */
  acceptOnly?: boolean;
  acceptDisabled?: boolean;
  /** Extra buttons after Cancel, for the period OK / Cancel / Apply row. */
  footerExtra?: ReactNode;
  width?: number;
};

/**
 * Modal dialog: fixed size, centred, OK/Cancel bottom-right in that order.
 * Focus moves in on open and returns to where it came from on close.
 */
export function Dialog({
  title,
  children,
  onClose,
  onAccept,
  acceptLabel = "OK",
  cancelLabel = "Cancel",
  acceptOnly,
  acceptDisabled,
  footerExtra,
  width = 380,
}: DialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const returnFocusTo = useRef<Element | null>(null);

  useEffect(() => {
    returnFocusTo.current = document.activeElement;
    dialogRef.current?.focus();

    return () => {
      const target = returnFocusTo.current;
      if (target instanceof HTMLElement) target.focus();
    };
  }, []);

  const accept = () => {
    if (acceptDisabled) return;
    if (onAccept) onAccept();
    else onClose();
  };

  return (
    <div className="dialog__scrim" role="presentation">
      <div
        ref={dialogRef}
        className="dialog"
        style={{ width }}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            onClose();
          } else if (e.key === "Enter" && e.target instanceof HTMLElement) {
            // Enter accepts, except from a multi-line field.
            if (e.target.tagName !== "TEXTAREA") {
              e.stopPropagation();
              accept();
            }
          }
        }}
      >
        <div className="dialog__titlebar">
          <Icon name="mushroom" size={13} />
          <span className="dialog__title">{title}</span>
          <Button className="dialog__close" onClick={onClose} aria-label="Close">
            ×
          </Button>
        </div>

        <div className="dialog__body selectable">{children}</div>

        <div className="dialog__footer">
          <Button onClick={accept} disabled={acceptDisabled} autoFocus>
            {acceptLabel}
          </Button>
          {acceptOnly ? null : (
            <Button onClick={onClose}>{cancelLabel}</Button>
          )}
          {footerExtra}
        </div>
      </div>
    </div>
  );
}
