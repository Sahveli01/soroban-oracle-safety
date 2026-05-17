"use client";

import {
  useScroll,
  useTransform,
  useMotionTemplate,
  useReducedMotion,
  type MotionValue,
} from "framer-motion";
import { type RefObject } from "react";

/**
 * The "page recedes under the next one" transform.
 *
 * Tracks a stacked panel's own scroll progress as it gets covered by
 * the next sticky panel: from the moment its top pins at the viewport
 * top (`start start`) to the moment its bottom reaches the viewport top
 * (`end start`). Over that range the panel scales down, dims and softly
 * blurs — so it reads as a page being slid beneath the next one.
 *
 * 100% scroll-linked: it is the scroll position, never an after-the-fact
 * snap or animation, so it can never fight the user (the prior `kasma`).
 *
 * Honors `prefers-reduced-motion`: identity transforms (no scale/blur),
 * the layout still stacks but nothing moves on its own.
 */
export function useRecede(ref: RefObject<HTMLElement | null>): {
  scale: MotionValue<number> | number;
  opacity: MotionValue<number> | number;
  filter: MotionValue<string> | string;
} {
  const reduced = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ["start start", "end start"],
  });

  const scale = useTransform(scrollYProgress, [0, 1], [1, 0.93]);
  const opacity = useTransform(scrollYProgress, [0, 0.85, 1], [1, 0.55, 0.35]);
  const blur = useTransform(scrollYProgress, [0, 1], [0, 4]);
  const filter = useMotionTemplate`blur(${blur}px)`;

  if (reduced) {
    return { scale: 1, opacity: 1, filter: "none" };
  }
  return { scale, opacity, filter };
}
