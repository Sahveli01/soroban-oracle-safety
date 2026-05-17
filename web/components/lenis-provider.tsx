"use client";

import { ReactLenis } from "lenis/react";
import type { ReactNode } from "react";

/**
 * Lenis smooth-scroll wrapper.
 *
 * Apple-grade scroll feel — slightly weighted, ~1.2s ease-out on
 * programmatic scrolls, lerp 0.1 on wheel input. Mounted at root so
 * anchor-link navigation in the pillow nav inherits the smoothing.
 *
 * Note: section snapping was tried (lenis/snap) and removed — proximity
 * snap fired after the user's scroll settled and animated to the nearest
 * section, which read as a stutter / loss of control. The "page-turn"
 * feel comes from full-height sections + parallax + smooth scroll, not
 * from snapping. Scroll stays fully user-controlled.
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
      {children}
    </ReactLenis>
  );
}
