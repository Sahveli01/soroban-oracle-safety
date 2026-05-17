"use client";

import { useEffect } from "react";
import { useLenis } from "lenis/react";
import Snap from "lenis/snap";

/**
 * Lenis Snap controller — the "notebook page-turn" mechanism.
 *
 * Native CSS `scroll-snap-type` does NOT work while Lenis controls the
 * root scroll (Lenis virtualizes wheel input). The official `lenis/snap`
 * module is the supported way to get section-to-section snapping, so we
 * register every `<section id>` as a snap target here.
 *
 * - `type: "proximity"` — only snaps when the scroll settles near a
 *   boundary; the user stays free to stop anywhere mid-section.
 * - Disabled on mobile (`max-width: 767px`) so native touch scroll is
 *   untouched, and on `prefers-reduced-motion: reduce`.
 * - Re-evaluates on media-query changes (breakpoint crossing, the user
 *   toggling reduced motion) and tears the instance down cleanly.
 *
 * Must render inside `<ReactLenis>` so `useLenis()` has context.
 * Renders nothing.
 */
export function SnapController() {
  const lenis = useLenis();

  useEffect(() => {
    if (!lenis) return;

    const mqMobile = window.matchMedia("(max-width: 767px)");
    const mqReduced = window.matchMedia("(prefers-reduced-motion: reduce)");

    let snap: Snap | null = null;

    const setup = () => {
      if (snap) {
        snap.destroy();
        snap = null;
      }
      // Desktop only, and never under reduced-motion.
      if (mqMobile.matches || mqReduced.matches) return;

      snap = new Snap(lenis, {
        type: "proximity",
        duration: 1.0,
        lerp: 0.1,
        debounce: 500,
      });

      document
        .querySelectorAll<HTMLElement>("section[id]")
        .forEach((el) => snap?.addElement(el, { align: "start" }));
    };

    setup();
    mqMobile.addEventListener("change", setup);
    mqReduced.addEventListener("change", setup);

    return () => {
      mqMobile.removeEventListener("change", setup);
      mqReduced.removeEventListener("change", setup);
      snap?.destroy();
    };
  }, [lenis]);

  return null;
}
