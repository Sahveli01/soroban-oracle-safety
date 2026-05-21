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
 * GESTURE MODEL — Pass 4B rewrite.
 *
 * The Pass-3 model treated lock release as `animDone && cooled &&
 * (now - lastWheel >= QUIET_GAP)`, where lastWheel was bumped by EVERY
 * wheel event including inertia. On a Mac trackpad, continuous use
 * emits a wheel event every ~16ms, so the quiet gap NEVER elapsed and
 * the lock only released on the 1600ms FAILSAFE — producing the
 * reported "7 swipes barely advances 2 pages" bug.
 *
 * The new model uses GESTURE BOUNDARIES: a fresh gesture is any wheel
 * event preceded by ≥GESTURE_GAP ms of silence. The inertia tail of a
 * previous swipe is a continuous stream (no gap) and is recognised as
 * the same gesture — silently swallowed. A second physical flick is
 * preceded by the brief lift between strokes (typically 60–200ms) and
 * is recognised as a new gesture.
 *
 * Behaviour during lock:
 *   - inertia tail of the in-flight gesture: ignored (no gap)
 *   - a NEW gesture (gap then event): direction queued
 * On lock release: queued direction fires immediately. Mouse notches
 * always read as new gestures (each notch is preceded by full silence).
 *
 * Tall slides are never clipped: content lives in a `.screen-min`
 * scroller; if it overflows, the wheel/touch scrolls *within it* until
 * it hits the edge, then the next gesture changes slide. Honors
 * prefers-reduced-motion (instant slide change).
 */

const SLIDE_MS = 420; // transition length (kept in sync with .deck-slide)
const GESTURE_GAP = 80; // wheel silent ≥ this ⇒ next event is a new gesture
const MIN_LOCK = 60; // tiny floor so one burst can't double-fire
const FAILSAFE = 1400; // absolute max lock — cannot deadlock
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
    let lastDelta = 0; // last |deltaY|, for velocity-spike new-gesture detection
    let pendingDir: 1 | -1 | 0 = 0; // queued direction (one slot, latest wins)
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

    const step = (dir: 1 | -1) => {
      // ===== INSTRUMENTATION START (Pass 4E — remove in 4F) =====
      console.log("[STEP]", {
        t: Math.round(performance.now()),
        dir,
        fromIndex: activeRef.current,
        toIndex: activeRef.current + dir,
      });
      // ===== INSTRUMENTATION END =====
      go(activeRef.current + dir);
    };

    goToIdImpl = (id: string) => {
      const idx = slides.findIndex((s) => s.id === id);
      if (idx >= 0) go(idx);
    };

    // Lock release: timestamp poll, no resettable timer ⇒ no deadlock.
    // No 'quiet' check here — inertia tail is filtered at the wheel
    // handler via gesture-gap detection, so the lock can release the
    // instant the animation finishes. If a fresh gesture queued during
    // the lock, fire it immediately on release.
    let pollRaf = 0;
    const poll = () => {
      if (locked) {
        const now = performance.now();
        const animDone = animEnd !== 0 && now - animEnd >= 0;
        const cooled = now - lockedAt >= MIN_LOCK;
        const failsafe = now - lockedAt >= FAILSAFE;
        if ((animDone && cooled) || failsafe) {
          locked = false;
          // ===== INSTRUMENTATION START (Pass 4E — remove in 4F) =====
          console.log("[LOCK_RELEASE]", {
            t: Math.round(now),
            pendingDir,
            via: failsafe ? "failsafe" : "animDone+cooled",
            willFire: pendingDir !== 0 ? `pending step(${pendingDir})` : "idle",
          });
          // ===== INSTRUMENTATION END =====
          if (pendingDir !== 0) {
            const d = pendingDir;
            pendingDir = 0;
            step(d);
          }
        }
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

      const now = performance.now();
      const dy = Math.abs(e.deltaY);
      const gap = now - lastWheel;
      // A fresh physical gesture is detected EITHER by silence (a
      // mouse notch, or the brief lift between two trackpad flicks
      // typically ≥80 ms) OR by a velocity spike (this event's
      // |deltaY| is ≥2× the last one — characteristic of a new flick
      // landing on top of the previous gesture's decaying inertia
      // tail). Inertia tail is uniformly small + monotonically
      // decaying, so neither rule fires for it.
      const isFreshGesture =
        gap >= GESTURE_GAP || (gap >= 20 && dy >= lastDelta * 2 && dy >= 6);

      // ===== INSTRUMENTATION START (Pass 4E — remove in 4F) =====
      console.log("[WHEEL]", {
        t: Math.round(now),
        dy: Math.round(dy),
        dir,
        gap: Math.round(gap),
        lastDelta: Math.round(lastDelta),
        locked,
        pendingDir,
        active: activeRef.current,
        isFresh: isFreshGesture,
        decision: !isFreshGesture
          ? "IGNORE-inertia"
          : locked
            ? `QUEUE→${dir}`
            : dy < WHEEL_MIN
              ? "IGNORE-tiny"
              : `STEP(${dir})`,
      });
      // ===== INSTRUMENTATION END =====

      lastWheel = now;
      lastDelta = dy;

      if (!isFreshGesture) return; // inertia tail — silently ignored

      if (locked) {
        // A fresh gesture during a transition isn't lost: latest one
        // wins (so user spamming flicks ends up advancing the right
        // way), and fires the moment the lock releases.
        pendingDir = dir;
        return;
      }

      if (Math.abs(e.deltaY) < WHEEL_MIN) return;
      step(dir);
    };

    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      const queue = (dir: 1 | -1) => {
        if (locked) pendingDir = dir;
        else step(dir);
      };
      switch (e.key) {
        case "ArrowDown":
        case "PageDown":
        case " ":
        case "Spacebar":
          e.preventDefault();
          queue(1);
          break;
        case "ArrowUp":
        case "PageUp":
          e.preventDefault();
          queue(-1);
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
      const dy = touchY - e.changedTouches[0].clientY;
      if (Math.abs(dy) < TOUCH_MIN) return;
      if (scrollerCanMove(e.changedTouches[0].target, touchDir)) return;
      const dir: 1 | -1 = dy > 0 ? 1 : -1;
      // Each touchend is a discrete gesture by construction (the
      // browser only emits one per finger lift) — no inertia filter
      // needed. During a transition, queue the direction so a quick
      // double-swipe doesn't drop the second flick.
      if (locked) {
        pendingDir = dir;
        return;
      }
      step(dir);
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
        transition: "transform 420ms cubic-bezier(0.22, 1, 0.36, 1)",
      }}
    />
  );
}
