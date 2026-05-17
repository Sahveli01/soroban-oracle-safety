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
 * Content is vertically centered via `min-h-screen flex justify-center`
 * — so short sections sit centered, but tall sections (architecture
 * sim, live, operator…) grow past the viewport and stay fully
 * scrollable/readable instead of being clipped behind a pinned box.
 */
export function SectionShell({ id, eyebrow, children }: SectionShellProps) {
  return (
    <section
      id={id}
      className="page-panel sticky top-0 bg-[var(--color-background)]"
    >
      <div className="mx-auto flex min-h-screen w-full max-w-5xl flex-col justify-center px-6 py-28 md:py-32">
        <motion.p
          initial={{ opacity: 0, y: 8 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5, ease: [0.19, 1, 0.22, 1] }}
          className="mb-10 font-mono text-xs uppercase tracking-[0.2em] text-text-muted"
        >
          {eyebrow}
        </motion.p>
        {children}
      </div>
    </section>
  );
}
