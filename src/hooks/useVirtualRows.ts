import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Which rows of a uniform-height list are worth rendering.
 *
 * The note list is every note you have. At 5,000 notes that is 5,000 DOM nodes
 * built on every render, which is what makes scrolling stutter — so only the
 * visible window plus a margin is rendered, and the rest is two spacer divs.
 *
 * Uniform height is what keeps this simple: both note rows and date headers
 * are exactly one row tall, so the first visible index is a division rather
 * than a measured offset.
 */
export type VirtualWindow = {
  /** Attach to the list element. The scroller is found from it. */
  ref: React.RefObject<HTMLDivElement | null>;
  /** First row to render. */
  start: number;
  /** One past the last row to render. */
  end: number;
  /** Pixels of blank space standing in for the rows above. */
  paddingTop: number;
  /** …and below. */
  paddingBottom: number;
};

/** Rows rendered beyond the viewport, so a flick does not show blank space. */
const OVERSCAN = 10;

/** The nearest ancestor that scrolls, or the document element. */
function scrollParent(from: HTMLElement): HTMLElement {
  let node: HTMLElement | null = from.parentElement;
  while (node) {
    const overflow = getComputedStyle(node).overflowY;
    if (overflow === "auto" || overflow === "scroll") return node;
    node = node.parentElement;
  }
  return document.documentElement;
}

export function useVirtualRows(
  total: number,
  rowHeight: number,
): VirtualWindow {
  const ref = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(0);

  const scroller = useRef<HTMLElement | null>(null);

  const measure = useCallback(() => {
    const element = scroller.current;
    if (!element) return;
    setScrollTop(element.scrollTop);
    setHeight(element.clientHeight);
  }, []);

  useEffect(() => {
    const list = ref.current;
    if (!list) return;

    // The list is not necessarily what scrolls — inside a Panel it is the
    // panel body. Finding the scroller instead of assuming it keeps the hook
    // usable anywhere, and avoids the silent failure where `scrollTop` is
    // always zero and every row renders anyway.
    const element = scrollParent(list);
    scroller.current = element;

    measure();
    element.addEventListener("scroll", measure, { passive: true });

    // The panel is resizable, so the visible count changes without scrolling.
    const observer = new ResizeObserver(measure);
    observer.observe(element);

    return () => {
      element.removeEventListener("scroll", measure);
      observer.disconnect();
    };
  }, [measure]);

  // Before the first measurement, render a screenful rather than nothing:
  // an empty list on the first paint would flash.
  const visible = height > 0 ? Math.ceil(height / rowHeight) : 40;

  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN);
  const end = Math.min(total, start + visible + OVERSCAN * 2);

  return {
    ref,
    start,
    end,
    paddingTop: start * rowHeight,
    paddingBottom: Math.max(0, (total - end) * rowHeight),
  };
}
