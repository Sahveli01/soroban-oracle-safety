"use client";

import { useEffect } from "react";
import { useLenis } from "lenis/react";

/**
 * Section pager — one gesture = exactly one section.
 *
 * The user explicitly wants discrete page-by-page navigation with
 * minimum stutter (a touchpad flick must advance ONE section, never
 * free-scroll, never double-jump on the momentum tail).
 *
 * How it stays jank-free:
 * - Lenis free scrolling is stopped; the ONLY way the page moves is one
 *   deterministic `lenis.scrollTo(section, { force, lock })` per gesture
 *   (eased ~0.95s). One controlled animation per intent → cannot fight
 *   the user, cannot half-finish.
 * - During the animation the input is locked; after it completes a
 *   momentum guard keeps the lock until wheel events quiesce for 160ms,
 *   so a touchpad's inertia tail can't trigger a second jump.
 * - Targets are every `section[id]` panel plus the footer, in DOM
 *   order. The current index is re-derived from scroll position each
 *   time, so nav-link jumps stay in sync.
 *
 * Keyboard (arrows / PageUp·Down / Space / Home / End) and touch swipe
 * are first-class. Honors prefers-reduced-motion (instant jump).
 *
 * Must render inside <ReactLenis> so useLenis() has context.
 */
const easeInOutQuart = (t: number) =>
  t < 0.5 ? 8 * t * t * t * t : 1 - Math.pow(-2 * t + 2, 4) / 2;

export function SectionPager() {
  const lenis = useLenis();

  useEffect(() => {
    if (!lenis) return;

    const reduced = window.matchMedia(
      "(prefers-reduced-motion: reduce)"
    ).matches;

    let locked = false;
    let animating = false;
    let unlockTimer: ReturnType<typeof setTimeout> | undefined;

    const targets = (): HTMLElement[] => {
      const secs = Array.from(
        document.querySelectorAll<HTMLElement>("main section[id]")
      );
      const footer = document.querySelector<HTMLElement>("footer");
      return footer ? [...secs, footer] : secs;
    };

    // Sections are `position: sticky` — their getBoundingClientRect is
    // distorted (pinned panels report top ≈ 0), which made backward
    // navigation resolve to the current position (no movement). offsetTop
    // up the offsetParent chain is the true *flow* position, unaffected
    // by sticky pinning, so it works identically in both directions.
    const docTop = (el: HTMLElement): number => {
      let y = 0;
      let n: HTMLElement | null = el;
      while (n) {
        y += n.offsetTop;
        n = n.offsetParent as HTMLElement | null;
      }
      return y;
    };

    const currentIndex = (list: HTMLElement[]): number => {
      const y = window.scrollY + 4;
      let idx = 0;
      list.forEach((el, i) => {
        if (docTop(el) <= y) idx = i;
      });
      return idx;
    };

    const scheduleUnlock = () => {
      if (unlockTimer) clearTimeout(unlockTimer);
      unlockTimer = setTimeout(() => {
        locked = false;
      }, 160);
    };

    const goTo = (index: number) => {
      const list = targets();
      const next = Math.min(list.length - 1, Math.max(0, index));
      const cur = currentIndex(list);
      if (next === cur) return;
      locked = true;
      animating = true;
      // Scroll to the numeric flow position (not the element — element
      // targets re-trigger the sticky-distorted rect inside Lenis).
      lenis.scrollTo(docTop(list[next]), {
        offset: 0,
        duration: reduced ? 0 : 0.95,
        immediate: reduced,
        lock: true,
        force: true,
        easing: easeInOutQuart,
        onComplete: () => {
          animating = false;
          scheduleUnlock();
        },
      });
    };

    const step = (dir: 1 | -1) => {
      const list = targets();
      goTo(currentIndex(list) + dir);
    };

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      if (Math.abs(e.deltaY) < 2) return;
      if (locked) {
        // Swallow the touchpad momentum tail; only re-arm the unlock
        // once the triggering animation itself has finished.
        if (!animating) scheduleUnlock();
        return;
      }
      step(e.deltaY > 0 ? 1 : -1);
    };

    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      const list = targets();
      switch (e.key) {
        case "ArrowDown":
        case "PageDown":
        case " ":
        case "Spacebar":
          e.preventDefault();
          if (!locked) step(1);
          break;
        case "ArrowUp":
        case "PageUp":
          e.preventDefault();
          if (!locked) step(-1);
          break;
        case "Home":
          e.preventDefault();
          if (!locked) goTo(0);
          break;
        case "End":
          e.preventDefault();
          if (!locked) goTo(list.length - 1);
          break;
        default:
          break;
      }
    };

    let touchStartY = 0;
    const onTouchStart = (e: TouchEvent) => {
      touchStartY = e.touches[0].clientY;
    };
    const onTouchMove = (e: TouchEvent) => {
      e.preventDefault();
    };
    const onTouchEnd = (e: TouchEvent) => {
      if (locked) return;
      const dy = touchStartY - e.changedTouches[0].clientY;
      if (Math.abs(dy) > 40) step(dy > 0 ? 1 : -1);
    };

    lenis.stop();
    window.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("keydown", onKey);
    window.addEventListener("touchstart", onTouchStart, { passive: true });
    window.addEventListener("touchmove", onTouchMove, { passive: false });
    window.addEventListener("touchend", onTouchEnd);

    return () => {
      if (unlockTimer) clearTimeout(unlockTimer);
      window.removeEventListener("wheel", onWheel);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("touchstart", onTouchStart);
      window.removeEventListener("touchmove", onTouchMove);
      window.removeEventListener("touchend", onTouchEnd);
      lenis.start();
    };
  }, [lenis]);

  return null;
}
