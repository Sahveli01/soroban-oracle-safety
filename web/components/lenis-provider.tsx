"use client";

import { ReactLenis } from "lenis/react";
import type { LenisOptions } from "lenis";
import { useMemo } from "react";
import type { ReactNode } from "react";

/**
 * Lenis smooth-scroll wrapper — free, continuous, buttery.
 *
 * History: an earlier build forced "one gesture = one section" via a
 * SectionPager that called `lenis.stop()` and intercepted every wheel
 * event. On macOS trackpads the inertia tail constantly rescheduled the
 * unlock timer, so the page locked permanently (couldn't scroll up,
 * felt delayed and janky). That whole mechanism is gone.
 *
 * Now: native free scroll, smoothed by Lenis with a lerp-based wheel
 * (no fixed duration → it tracks the gesture 1:1 and glides to rest,
 * never fights the user, identical feel in both directions). The
 * "page-turn" is purely the stacked `position: sticky` panels sliding
 * over each other — see SectionShell / globals.css `.page-panel`. The
 * scroll engine no longer knows or cares about sections.
 *
 * Honors prefers-reduced-motion by dropping smoothing entirely.
 */
export function LenisProvider({ children }: { children: ReactNode }) {
  const options = useMemo<LenisOptions>(() => {
    const reduced =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    if (reduced) {
      // No smoothing — instant native scroll, programmatic jumps snap.
      return { smoothWheel: false, syncTouch: false, autoRaf: true };
    }

    return {
      // lerp (not duration) → continuous gesture-tracking glide. 0.09 is
      // the sweet spot: smooth enough to feel premium, responsive enough
      // to never feel laggy or "delayed after the gesture".
      lerp: 0.09,
      smoothWheel: true,
      wheelMultiplier: 1,
      // Native momentum on touch already feels great; syncTouch adds lag.
      syncTouch: false,
      touchMultiplier: 1.6,
      gestureOrientation: "vertical",
      overscroll: false,
      autoRaf: true,
    };
  }, []);

  return (
    <ReactLenis root options={options}>
      {children}
    </ReactLenis>
  );
}
