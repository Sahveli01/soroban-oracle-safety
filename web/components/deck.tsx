"use client";

import { useEffect, useState, type ReactNode } from "react";
import useEmblaCarousel from "embla-carousel-react";
import { WheelGesturesPlugin } from "embla-carousel-wheel-gestures";

/**
 * Presentation deck — vertical carousel powered by Embla.
 *
 * Pass 6 rewrite. After Pass 4B–5C spent five iterations tuning a
 * custom wheel/touch handler against trackpad inertia and never
 * fully resolving Mac under-advance + Windows over-advance, the
 * gesture-detection approach was abandoned in favour of a
 * production gesture engine. Embla is the carousel used by Vercel,
 * Shopify, Linear, and Resend — its wheel-gestures plugin captures
 * native momentum on every platform and applies a single, consistent
 * animation curve. Custom gesture code = custom gesture bugs;
 * Embla = zero custom gesture code.
 *
 * The public API is unchanged so Nav and any other consumer keeps
 * working untouched:
 *   - DeckSlide { id, node }
 *   - Deck({ slides }) component
 *   - deckGoTo(id)  — module-level bridge for Nav clicks
 *   - useDeckIndex() — listens to the `deck:change` event we re-emit
 *
 * What's gone vs Pass 5C:
 *   GESTURE_GAP, SLIDE_MS, MIN_LOCK, FAILSAFE, WHEEL_MIN, TOUCH_MIN
 *   lastWheel / lastDelta / pendingDir / isFreshGesture
 *   the requestAnimationFrame poll loop and lock state machine
 *   manual touchstart/touchmove/touchend handlers
 *   the "Pass 4F gap > 150 ms" guard at lock release
 *   per-slide translateY(0|100%) inline transform
 *
 * What's preserved:
 *   - DeckRail top progress bar (still emits scaleX based on index)
 *   - Keyboard navigation (Arrow/Page/Home/End/Space)
 *   - Nested .screen-min scroll for sections that overflow — handled
 *     by a capture-phase wheel listener that stops propagation when
 *     the inner scroller has slack in the gesture direction (so the
 *     browser scrolls .screen-min instead of Embla changing slide).
 *   - prefers-reduced-motion → duration: 0 (instant snap)
 */

export interface DeckSlide {
  id: string;
  node: ReactNode;
}

// Module-level bridge so external components (Nav) can drive the
// carousel without prop-drilling or context. The current Deck instance
// installs the impl on mount and clears it on unmount.
let scrollToIdImpl: ((id: string) => void) | null = null;
export function deckGoTo(id: string): void {
  scrollToIdImpl?.(id);
}

/**
 * Read the deck's current slide index. Subscribes to the same
 * "deck:change" event the Deck emits on every navigation, so the nav
 * highlights the active slide without any shared state.
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
  const [reduce, setReduce] = useState(false);
  useEffect(() => {
    setReduce(window.matchMedia("(prefers-reduced-motion: reduce)").matches);
  }, []);

  const [emblaRef, emblaApi] = useEmblaCarousel(
    {
      axis: "y",
      loop: false,
      align: "start",
      // Snap to slides (no drag-free), don't skip past on hard flick.
      skipSnaps: false,
      dragFree: false,
      // Lower = snappier. Pass 8: 22 → 14 after user + friends reported
      // perceptible gesture→snap lag on BOTH Windows and Mac. 14 is
      // ~35 % faster than Pass 6's 22; Reels/TikTok-class snap feel.
      // If it ever reads as too aggressive, 16 is the next stop up.
      duration: reduce ? 0 : 14,
      containScroll: "trimSnaps",
      // Treat a slide as in-view (and thus 'selected') once 70 % of it
      // crosses the viewport — tightens active-index reporting so
      // listeners like Nav highlight the right slide during the snap.
      inViewThreshold: 0.7,
    },
    [
      WheelGesturesPlugin({
        forceWheelAxis: "y",
      }),
    ]
  );

  const [activeIndex, setActiveIndex] = useState(0);

  // Subscribe to Embla's selection events and re-emit as `deck:change`
  // so useDeckIndex (and any other listener) sees a stable contract.
  useEffect(() => {
    if (!emblaApi) return;
    const onSelect = () => {
      const idx = emblaApi.selectedScrollSnap();
      setActiveIndex(idx);
      window.dispatchEvent(
        new CustomEvent("deck:change", {
          detail: { index: idx, id: slides[idx]?.id, total: slides.length },
        })
      );
    };
    emblaApi.on("select", onSelect);
    onSelect();
    return () => {
      emblaApi.off("select", onSelect);
    };
  }, [emblaApi, slides]);

  // Install the deckGoTo bridge so Nav clicks resolve to Embla.scrollTo.
  useEffect(() => {
    if (!emblaApi) return;
    scrollToIdImpl = (id: string) => {
      const idx = slides.findIndex((s) => s.id === id);
      if (idx >= 0) emblaApi.scrollTo(idx);
    };
    return () => {
      scrollToIdImpl = null;
    };
  }, [emblaApi, slides]);

  // Keyboard navigation. Mirrors the old API exactly.
  useEffect(() => {
    if (!emblaApi) return;
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
          emblaApi.scrollNext();
          break;
        case "ArrowUp":
        case "PageUp":
          e.preventDefault();
          emblaApi.scrollPrev();
          break;
        case "Home":
          e.preventDefault();
          emblaApi.scrollTo(0);
          break;
        case "End":
          e.preventDefault();
          emblaApi.scrollTo(slides.length - 1);
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [emblaApi, slides.length]);

  // Nested-scroll respect. The Embla wheel plugin listens at the
  // viewport; child wheel events bubble up to it. When a wheel happens
  // inside a `.screen-min` scroller that still has slack in the gesture
  // direction, stop propagation so the browser scrolls the inner
  // element natively and Embla never sees the event. We DO NOT
  // preventDefault — the native scroll must still happen.
  useEffect(() => {
    if (!emblaApi) return;
    const onWheelCapture = (e: WheelEvent) => {
      let el = e.target as HTMLElement | null;
      while (el && !el.classList?.contains("screen-min")) {
        el = el.parentElement;
      }
      if (!el) return;
      const slack = el.scrollHeight - el.clientHeight;
      if (slack <= 2) return;
      const dir = e.deltaY > 0 ? 1 : -1;
      const canScrollWithin =
        dir === 1 ? el.scrollTop < slack - 1 : el.scrollTop > 1;
      if (canScrollWithin) e.stopPropagation();
    };
    window.addEventListener("wheel", onWheelCapture, {
      capture: true,
      passive: true,
    });
    return () => {
      window.removeEventListener("wheel", onWheelCapture, { capture: true });
    };
  }, [emblaApi]);

  return (
    <>
      <div className="deck-viewport" ref={emblaRef}>
        <div className="deck-container">
          {slides.map((s, i) => (
            <div
              key={s.id}
              className="deck-slide"
              aria-hidden={i !== activeIndex}
              inert={i !== activeIndex ? true : undefined}
            >
              {s.node}
            </div>
          ))}
        </div>
      </div>
      <DeckRail active={activeIndex} total={slides.length} />
    </>
  );
}

/** Top progress rail — driven by slide index, not scroll. */
function DeckRail({ active, total }: { active: number; total: number }) {
  return (
    <div
      className="scroll-rail"
      style={{
        transform: `scaleX(${total > 1 ? active / (total - 1) : 0})`,
        transition: "transform 320ms cubic-bezier(0.22, 1, 0.36, 1)",
      }}
    />
  );
}
