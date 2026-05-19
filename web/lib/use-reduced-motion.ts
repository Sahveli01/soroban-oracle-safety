"use client";

import { useEffect, useState } from "react";

/**
 * Returns true if the user has requested reduced motion at the OS
 * level (prefers-reduced-motion: reduce). Reactive — updates without a
 * page reload if the preference is toggled.
 *
 * SSR-safe: starts false on the server and on first client paint, then
 * resolves in an effect (avoids a hydration mismatch).
 */
export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduced(mq.matches);

    const handler = (e: MediaQueryListEvent) => setReduced(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  return reduced;
}
