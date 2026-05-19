"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";

/**
 * Presentation deck — the PowerPoint model.
 *
 * Why this finally ends the recurring "can't go up / lands halfway"
 * saga: there is NO document scroll anymore. The source of truth is an
 * integer slide index. Every input (mouse wheel, trackpad swipe,
 * arrow/PageUp·Down/Home/End, touch swipe, nav click) just does
 * `index ± 1`. A CSS transform animates to that slide and always runs
 * to completion. An index cannot be "half", and "up" is literally the
 * same code as "down" with the sign flipped — so backward is provably
 * identical to forward and the old asymmetry is impossible.
 *
 * The beloved page-turn is preserved verbatim: slides are stacked
 * absolutely; the incoming slide (higher z-index) slides up from
 * translateY(100%) and COVERS the current one (which stays put), and
 * going back the current slide slides down to reveal the previous one
 * underneath. Same visual as the old sticky cover, now deterministic.
 *
 * Gesture model (one gesture = one slide, mouse AND trackpad):
 * the first wheel event of an idle moment fires once; everything after
 * (the rest of a trackpad burst + its inertia tail) is swallowed until
 * the transition has finished AND the wheel has been quiet — released
 * by a timestamp poll that cannot deadlock (hard failsafe). A mouse
 * notch and a trackpad flick therefore both advance exactly one slide.
 *
 * Tall slides are never clipped: content lives in a `.screen-min`
 * scroller; if it overflows, the wheel/touch scrolls *within it* until
 * it hits the edge, then the next gesture changes slide. Honors
 * prefers-reduced-motion (instant slide change).
 */

const SLIDE_MS = 520; // transition length (kept in sync with .deck-slide)
const QUIET_GAP = 80; // wheel silent this long ⇒ trackpad inertia ended
const MIN_LOCK = 60; // tiny floor so one burst can't double-fire
const FAILSAFE = 1600; // absolute max lock — cannot deadlock
const WHEEL_MIN = 4;
const TOUCH_MIN = 45;

export interface DeckSlide {
  id: string;
  node: ReactNode;
}

// Imperative bridge so the nav can drive the deck.
let goToIdImpl: ((id: string) => void) | null = null;
export function deckGoTo(id: string): void {
  goToIdImpl?.(id);
}

/**
 * Read the deck's current slide index. Subscribes to the same
 * "deck:change" event the Deck already emits on every navigation, so
 * the nav highlights the active slide without any new shared state.
 * (Missing the Deck's initial emit(0) is harmless — index defaults to
 * 0; every subsequent change is captured.)
 */
export function useDeckIndex(): number {
  const [index, setIndex] = useState(0);
  useEffect(() => {
    const onChange = (e: Event) => {
      const detail = (e as CustomEvent<{ index: number }>).detail;
      if (detail && typeof detail.index === "number") setIndex(detail.index);
    };
    window.addEventListener("deck:change", onChange);
    return () => window.removeEventListener("deck:change", onChange);
  }, []);
  return index;
}

export function Deck({ slides }: { slides: DeckSlide[] }) {
  const [active, setActive] = useState(0);
  const activeRef = useRef(0);
  const count = slides.length;

  useEffect(() => {
    const reduce = window.matchMedia(
      "(prefers-reduced-motion: reduce)"
    ).matches;

    let locked = false;
    let lockedAt = 0;
    let animEnd = 0;
    let lastWheel = 0;
    let endTimer: ReturnType<typeof setTimeout> | undefined;

    const emit = (i: number) => {
      window.dispatchEvent(
        new CustomEvent("deck:change", {
          detail: { index: i, id: slides[i]?.id, total: count },
        })
      );
    };
    emit(0);

    const go = (next: number) => {
      const cur = activeRef.current;
      const target = Math.min(count - 1, Math.max(0, next));
      locked = true;
      lockedAt = performance.now();
      animEnd = 0;
      if (endTimer) clearTimeout(endTimer);
      endTimer = setTimeout(
        () => {
          animEnd = performance.now();
        },
        reduce ? 0 : SLIDE_MS
      );
      if (target === cur) return; // at an edge — still swallow inertia
      activeRef.current = target;
      setActive(target);
      emit(target);
    };

    const step = (dir: 1 | -1) => go(activeRef.current + dir);

    goToIdImpl = (id: string) => {
      const idx = slides.findIndex((s) => s.id === id);
      if (idx >= 0) go(idx);
    };

    // Lock release: timestamp poll, no resettable timer ⇒ no deadlock.
    let pollRaf = 0;
    const poll = () => {
      if (locked) {
        const now = performance.now();
        const animDone = animEnd !== 0 && now - animEnd >= 0;
        const cooled = now - lockedAt >= MIN_LOCK;
        const quiet = now - lastWheel >= QUIET_GAP;
        const failsafe = now - lockedAt >= FAILSAFE;
        if ((animDone && cooled && quiet) || failsafe) locked = false;
      }
      pollRaf = requestAnimationFrame(poll);
    };
    pollRaf = requestAnimationFrame(poll);

    // Find an overflowing in-slide scroller under the pointer, so tall
    // sections scroll internally before the gesture flips the slide.
    const scrollerCanMove = (
      target: EventTarget | null,
      dir: 1 | -1
    ): boolean => {
      let el = target as HTMLElement | null;
      while (el && !el.classList?.contains("screen-min")) {
        el = el.parentElement;
      }
      if (!el) return false;
      const slack = el.scrollHeight - el.clientHeight;
      if (slack <= 2) return false;
      if (dir === 1) return el.scrollTop < slack - 1;
      return el.scrollTop > 1;
    };

    const onWheel = (e: WheelEvent) => {
      const dir: 1 | -1 = e.deltaY > 0 ? 1 : -1;
      // Let a tall slide scroll within itself first.
      if (scrollerCanMove(e.target, dir)) return;
      e.preventDefault();
      lastWheel = performance.now();
      if (locked) return;
      if (Math.abs(e.deltaY) < WHEEL_MIN) return;
      step(dir);
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
          if (!locked) go(0);
          break;
        case "End":
          e.preventDefault();
          if (!locked) go(count - 1);
          break;
        default:
          break;
      }
    };

    let touchY = 0;
    let touchDir: 1 | -1 = 1;
    const onTouchStart = (e: TouchEvent) => {
      touchY = e.touches[0].clientY;
    };
    const onTouchMove = (e: TouchEvent) => {
      const dir: 1 | -1 = e.touches[0].clientY < touchY ? 1 : -1;
      touchDir = dir;
      if (scrollerCanMove(e.target, dir)) return;
      e.preventDefault();
    };
    const onTouchEnd = (e: TouchEvent) => {
      lastWheel = performance.now();
      if (locked) return;
      const dy = touchY - e.changedTouches[0].clientY;
      if (Math.abs(dy) < TOUCH_MIN) return;
      if (scrollerCanMove(e.changedTouches[0].target, touchDir)) return;
      step(dy > 0 ? 1 : -1);
    };

    window.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("keydown", onKey);
    window.addEventListener("touchstart", onTouchStart, { passive: true });
    window.addEventListener("touchmove", onTouchMove, { passive: false });
    window.addEventListener("touchend", onTouchEnd);

    return () => {
      goToIdImpl = null;
      cancelAnimationFrame(pollRaf);
      if (endTimer) clearTimeout(endTimer);
      window.removeEventListener("wheel", onWheel);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("touchstart", onTouchStart);
      window.removeEventListener("touchmove", onTouchMove);
      window.removeEventListener("touchend", onTouchEnd);
    };
  }, [count, slides]);

  return (
    <div className="deck-root">
      {slides.map((s, i) => (
        <div
          key={s.id}
          className="deck-slide"
          aria-hidden={i !== active}
          inert={i !== active ? true : undefined}
          style={{
            zIndex: i,
            transform: `translate3d(0, ${i <= active ? 0 : 100}%, 0)`,
          }}
        >
          {s.node}
        </div>
      ))}
      <DeckRail active={active} total={count} />
    </div>
  );
}

/** Top progress rail — driven by slide index, not scroll. */
function DeckRail({ active, total }: { active: number; total: number }) {
  return (
    <div
      className="scroll-rail"
      style={{
        transform: `scaleX(${total > 1 ? active / (total - 1) : 0})`,
        transition: "transform 520ms cubic-bezier(0.22, 1, 0.36, 1)",
      }}
    />
  );
}
