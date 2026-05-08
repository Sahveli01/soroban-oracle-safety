"use client";

import { ReactLenis } from "lenis/react";
import type { ReactNode } from "react";

/**
 * Lenis smooth-scroll wrapper.
 *
 * Apple-grade scroll feel — slightly weighted, ~1.2s ease-out on
 * programmatic scrolls, lerp 0.1 on wheel input. Mounted at root so
 * anchor-link navigation in the pillow nav inherits the smoothing.
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
