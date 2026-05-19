"use client";

import { motion } from "framer-motion";
import { type ReactNode } from "react";

interface SectionShellProps {
  id: string;
  eyebrow: string;
  children: ReactNode;
}

/**
 * Stacked section panel — the "notebook page-turn", done robustly.
 *
 * Each section is a `sticky top-0` panel with an opaque background and a
 * rounded, soft-shadowed leading edge (`.page-panel`). Consecutive
 * sticky siblings mean the next panel simply slides up and covers this
 * one — that cover IS the page-turn.
 *
 * Deliberately ZERO scroll-driven transforms (no scale / blur / opacity
 * on scroll). The previous attempt did that and it made text blurry,
 * animations look broken, and the transition feel half-finished. This
 * is pure CSS sticky: it cannot jank, and it never fights the scroll.
 *
 * Content is TOP-biased via `.screen-min` (100vh→100svh) + flex
 * justify-start with a fluid clamp() top offset (~15vh) and a smaller
 * bottom pad — so short sections anchor against a consistent optical
 * baseline (Vercel/Linear-style) with residual whitespace BELOW as
 * intentional breathing room, instead of being centred into two
 * awkward symmetric voids. Tall sections (architecture sim, live,
 * operator…) grow past the viewport and stay scrollable/readable.
 */
export function SectionShell({ id, eyebrow, children }: SectionShellProps) {
  return (
    <section
      id={id}
      className="page-panel relative h-full bg-[var(--color-background)]"
    >
      <div className="screen-min mx-auto flex w-full max-w-5xl flex-col justify-start px-6 pt-[clamp(4.5rem,15vh,8rem)] pb-[clamp(2.5rem,6vh,4rem)]">
        <motion.p
          initial={{ opacity: 0, y: 8 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5, ease: [0.19, 1, 0.22, 1] }}
          className="t-eyebrow mb-8"
        >
          {eyebrow}
        </motion.p>
        {children}
      </div>
    </section>
  );
}
