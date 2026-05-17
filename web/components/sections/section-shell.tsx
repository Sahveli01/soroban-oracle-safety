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
 * Content is vertically centered via `.screen-min` (100vh→100svh) +
 * flex justify-center with fluid clamp() padding — so short sections
 * sit centered and fit the *visible* viewport (no macOS Safari 100vh
 * overflow), while tall sections (architecture sim, live, operator…)
 * grow past it and stay fully scrollable/readable.
 */
export function SectionShell({ id, eyebrow, children }: SectionShellProps) {
  return (
    <section
      id={id}
      className="page-panel relative h-full bg-[var(--color-background)]"
    >
      <div className="screen-min mx-auto flex w-full max-w-5xl flex-col justify-center px-6 py-[clamp(4rem,9vh,7rem)]">
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
