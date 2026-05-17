"use client";

import { useEffect } from "react";

/**
 * Section-snap scroll controller — buttery, page-by-page, both ways.
 *
 * The user wants: one trackpad gesture / one arrow key = advance
 * exactly one full, perfectly-aligned section, forward AND backward,
 * with a smooth eased transition (no scrollbar, no free scrolling).
 *
 * Why this is robust where the old SectionPager wasn't:
 *
 * 1. NO Lenis. We own the scroll: a single rAF tween (easeInOutCubic,
 *    ~750ms) drives `window.scrollTo`. The page-turn visual is still
 *    the stacked `position: sticky` panels — this only changes WHERE
 *    the real window scroll lands, so sticky keeps resolving against
 *    the viewport exactly as before.
 *
 * 2. The lock can NEVER deadlock. The old pager rescheduled its
 *    unlock timer on every wheel event, so a macOS trackpad's inertia
 *    tail kept it locked forever (couldn't scroll up). Here the lock
 *    is released by a timestamp poll: unlock once the animation has
 *    settled (min cooldown) AND the wheel has been quiet long enough
 *    (inertia genuinely ended). Inertia is finite, so it always
 *    releases; a hard failsafe covers any pathological case.
 *
 * 3. Targets are flow positions (`docTop` — summed offsetTop up the
 *    offsetParent chain), unaffected by sticky pinning, so forward and
 *    backward navigation are perfectly symmetric and aligned.
 *
 * 4. Tall sections are never clipped: if a panel is taller than the
 *    viewport, a gesture first pages *within* it by one viewport, and
 *    only the boundary step crosses to the next/previous section.
 *
 * Honors prefers-reduced-motion (instant jump, still page-by-page).
 */

const ANIM_MS = 750;
const MIN_COOLDOWN = 200; // floor after the tween before a new gesture
const QUIET_GAP = 110; // wheel must be silent this long (= inertia ended)
const FAILSAFE_MS = 2600; // absolute max lock — cannot deadlock
const WHEEL_MIN = 3;
const TOUCH_MIN = 45;
const EDGE_TOL = 8;

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/** True flow position of an element, unaffected by `position: sticky`. */
function docTop(el: HTMLElement): number {
  let y = 0;
  let n: HTMLElement | null = el;
  while (n) {
    y += n.offsetTop;
    n = n.offsetParent as HTMLElement | null;
  }
  return y;
}

// Imperative bridge so the nav can drive the same eased engine.
let scrollToIdImpl: ((id: string) => void) | null = null;
export function scrollToSection(id: string): void {
  scrollToIdImpl?.(id);
}

export function SectionSnap() {
  useEffect(() => {
    const reduce = window.matchMedia(
      "(prefers-reduced-motion: reduce)"
    ).matches;

    const targets = (): HTMLElement[] => {
      const secs = Array.from(
        document.querySelectorAll<HTMLElement>("main section[id]")
      );
      const footer = document.querySelector<HTMLElement>("footer");
      return footer ? [...secs, footer] : secs;
    };

    const maxScroll = () =>
      Math.max(
        0,
        document.documentElement.scrollHeight - window.innerHeight
      );

    let locked = false;
    let lockedAt = 0;
    let animEndTime = 0;
    let lastWheelTime = 0;
    let raf = 0;

    const startLock = () => {
      locked = true;
      lockedAt = performance.now();
      animEndTime = 0;
    };

    const animateTo = (toRaw: number) => {
      const to = Math.max(0, Math.min(toRaw, maxScroll()));
      const from = window.scrollY;
      if (Math.abs(to - from) < 2) {
        animEndTime = performance.now();
        return;
      }
      if (reduce) {
        window.scrollTo(0, to);
        animEndTime = performance.now();
        return;
      }
      const start = performance.now();
      const tick = () => {
        const p = Math.min(1, (performance.now() - start) / ANIM_MS);
        window.scrollTo(0, from + (to - from) * easeInOutCubic(p));
        if (p < 1) {
          raf = requestAnimationFrame(tick);
        } else {
          window.scrollTo(0, to);
          animEndTime = performance.now();
        }
      };
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(tick);
    };

    /**
     * Next aligned scroll target. Pages within a tall section by one
     * viewport; otherwise crosses to the adjacent section. Symmetric
     * up/down so backward feels identical to forward.
     */
    const computeTarget = (dir: 1 | -1): number => {
      const list = targets();
      if (!list.length) return window.scrollY;

      const y = window.scrollY;
      const vh = window.innerHeight;
      const tops = list.map(docTop);
      const end = maxScroll();

      let i = 0;
      for (let k = 0; k < tops.length; k++) {
        if (tops[k] <= y + EDGE_TOL) i = k;
      }

      const top = tops[i];
      const bottom = i + 1 < tops.length ? tops[i + 1] : end + vh;
      // Furthest scroll still "inside" this section (tall-section paging).
      const innerMax = Math.max(top, Math.min(bottom - vh, end));

      if (dir === 1) {
        if (y < innerMax - EDGE_TOL) return Math.min(y + vh, innerMax);
        return i + 1 < tops.length ? tops[i + 1] : end;
      }

      if (y > top + EDGE_TOL) return Math.max(y - vh, top);
      if (i > 0) {
        const pTop = tops[i - 1];
        const pInnerMax = Math.max(pTop, Math.min(top - vh, end));
        // Land on the previous section's bottom-aligned page if it's
        // tall, otherwise its top. Reading upward stays continuous.
        return pInnerMax;
      }
      return 0;
    };

    const step = (dir: 1 | -1) => {
      startLock();
      animateTo(computeTarget(dir));
    };

    const goToId = (id: string) => {
      const el = document.getElementById(id);
      if (!el) return;
      startLock();
      animateTo(docTop(el));
    };
    scrollToIdImpl = goToId;

    // Timestamp poll — releases the lock without any resettable timer.
    let pollRaf = 0;
    const poll = () => {
      if (locked) {
        const now = performance.now();
        const animDone =
          animEndTime !== 0 && now - animEndTime >= MIN_COOLDOWN;
        const quiet = now - lastWheelTime >= QUIET_GAP;
        const failsafe = now - lockedAt >= FAILSAFE_MS;
        if ((animDone && quiet) || failsafe) locked = false;
      }
      pollRaf = requestAnimationFrame(poll);
    };
    pollRaf = requestAnimationFrame(poll);

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      lastWheelTime = performance.now();
      if (locked) return;
      if (Math.abs(e.deltaY) < WHEEL_MIN) return;
      step(e.deltaY > 0 ? 1 : -1);
    };

    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
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
          if (!locked) {
            startLock();
            animateTo(0);
          }
          break;
        case "End":
          e.preventDefault();
          if (!locked) {
            startLock();
            animateTo(maxScroll());
          }
          break;
        default:
          break;
      }
    };

    let touchY = 0;
    const onTouchStart = (e: TouchEvent) => {
      touchY = e.touches[0].clientY;
    };
    const onTouchMove = (e: TouchEvent) => {
      e.preventDefault();
    };
    const onTouchEnd = (e: TouchEvent) => {
      lastWheelTime = performance.now();
      if (locked) return;
      const dy = touchY - e.changedTouches[0].clientY;
      if (Math.abs(dy) > TOUCH_MIN) step(dy > 0 ? 1 : -1);
    };

    window.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("keydown", onKey);
    window.addEventListener("touchstart", onTouchStart, { passive: true });
    window.addEventListener("touchmove", onTouchMove, { passive: false });
    window.addEventListener("touchend", onTouchEnd);

    return () => {
      scrollToIdImpl = null;
      cancelAnimationFrame(raf);
      cancelAnimationFrame(pollRaf);
      window.removeEventListener("wheel", onWheel);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("touchstart", onTouchStart);
      window.removeEventListener("touchmove", onTouchMove);
      window.removeEventListener("touchend", onTouchEnd);
    };
  }, []);

  return null;
}
