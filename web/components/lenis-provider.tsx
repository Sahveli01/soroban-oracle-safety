"use client";

import { ReactLenis } from "lenis/react";
import type { ReactNode } from "react";

import { SectionPager } from "./section-pager";

/**
 * Lenis smooth-scroll wrapper.
 *
 * Lenis powers the eased programmatic scrolls. Free wheel/touch
 * scrolling is taken over by `SectionPager` (rendered inside, so it has
 * Lenis context): one gesture = exactly one section, no free-scroll,
 * minimum stutter. Lenis itself is paused by the pager and driven only
 * through `scrollTo(..., { force, lock })`.
 *
 * (An earlier `lenis/snap` attempt fired a snap AFTER the scroll
 * settled — that felt like a stutter and was removed. This pager acts
 * on the gesture itself, never after it.)
 */
export function LenisProvider({ children }: { children: ReactNode }) {
  return (
    <ReactLenis
      root
      options={{
        lerp: 0.1,
        duration: 1.2,
        smoothWheel: true,
        wheelMultiplier: 1,
        touchMultiplier: 2,
      }}
    >
      <SectionPager />
      {children}
    </ReactLenis>
  );
}
