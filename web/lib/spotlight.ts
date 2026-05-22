import type { MouseEvent as ReactMouseEvent } from "react";

/**
 * Cursor-tracking handler for the .spotlight CSS class (defined in
 * app/globals.css). Writes the cursor position into --mx / --my so
 * the radial-gradient on .spotlight::before tracks the mouse.
 *
 * Pair with `className="spotlight"` and `onMouseMove={trackSpotlight}`
 * on any link/card you want the cursor-following glow on.
 */
export function trackSpotlight(e: ReactMouseEvent<HTMLElement>): void {
  const el = e.currentTarget;
  const rect = el.getBoundingClientRect();
  el.style.setProperty("--mx", `${e.clientX - rect.left}px`);
  el.style.setProperty("--my", `${e.clientY - rect.top}px`);
}
