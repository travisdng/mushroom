import { useCallback, useRef, useState } from "react";

type SplitterProps = {
  orientation: "vertical" | "horizontal";
  /** Called with the pixel delta since the last move while dragging. */
  onDrag: (delta: number) => void;
  onDragEnd?: () => void;
  label: string;
};

/**
 * A 4px draggable divider. Resizes live (R4.3); the panes it sits between own
 * their own minimum sizes and clamp accordingly.
 */
export function Splitter({
  orientation,
  onDrag,
  onDragEnd,
  label,
}: SplitterProps) {
  const [dragging, setDragging] = useState(false);
  const last = useRef(0);

  const onPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      e.currentTarget.setPointerCapture(e.pointerId);
      last.current = orientation === "vertical" ? e.clientX : e.clientY;
      setDragging(true);
    },
    [orientation],
  );

  const onPointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!dragging) return;
      const current = orientation === "vertical" ? e.clientX : e.clientY;
      const delta = current - last.current;
      if (delta !== 0) {
        last.current = current;
        onDrag(delta);
      }
    },
    [dragging, orientation, onDrag],
  );

  const stop = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!dragging) return;
      e.currentTarget.releasePointerCapture(e.pointerId);
      setDragging(false);
      onDragEnd?.();
    },
    [dragging, onDragEnd],
  );

  return (
    <div
      className={`splitter splitter--${orientation}`}
      data-dragging={dragging ? "true" : undefined}
      role="separator"
      aria-label={label}
      aria-orientation={orientation === "vertical" ? "vertical" : "horizontal"}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={stop}
      onPointerCancel={stop}
    />
  );
}
