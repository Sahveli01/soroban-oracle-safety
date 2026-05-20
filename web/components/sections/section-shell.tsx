"use client";

import { motion } from "framer-motion";
import { type ReactNode } from "react";

/**
 * Per-section content density. The deck slide is a fixed 100svh
 * viewport; different sections carry very different content loads, so a
 * single one-size-fits-all padding/spacing leaves dense sections
 * clipping at the bottom and light sections drowning in dead space.
 *
 *   comfortable — short content that needs to sit lower in the
 *                 viewport (Live: 2 short lists, otherwise bottom-heavy)
 *   default     — light/medium content (Attack, How-It-Works, Infra…)
 *   dense       — 5–6 rows or 5+ cards (Solution, Mechanism, Trust)
 *   compact     — the heaviest single sections (Architecture)
 *
 * Top padding is always larger than bottom — a top-biased baseline reads
 * as deliberate composition (Vercel/Linear voice) rather than vertical
 * centring, and matches the rest of the system.
 */
type Density = "comfortable" | "default" | "dense" | "compact";

interface SectionShellProps {
  id: string;
  eyebrow: string;
  density?: Density;
  children: ReactNode;
}

const DENSITY: Record<Density, { pad: string; gap: string }> = {
  comfortable: {
    pad: "pt-[clamp(5.5rem,18vh,9rem)] pb-[clamp(3rem,7vh,5rem)]",
    gap: "mb-9",
  },
  default: {
    pad: "pt-[clamp(4.5rem,15vh,8rem)] pb-[clamp(2.5rem,6vh,4rem)]",
    gap: "mb-8",
  },
  dense: {
    pad: "pt-[clamp(3rem,10vh,5.5rem)] pb-[clamp(2rem,5vh,3.5rem)]",
    gap: "mb-6",
  },
  compact: {
    pad: "pt-[clamp(2.5rem,8vh,4.5rem)] pb-[clamp(1.5rem,4vh,3rem)]",
    gap: "mb-5",
  },
};

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
 */
export function SectionShell({
  id,
  eyebrow,
  density = "default",
  children,
}: SectionShellProps) {
  const d = DENSITY[density];
  return (
    <section
      id={id}
      className="page-panel relative h-full bg-[var(--color-background)]"
    >
      <div
        className={`screen-min mx-auto flex w-full max-w-5xl flex-col justify-start px-6 ${d.pad}`}
      >
        <motion.p
          initial={{ opacity: 0, y: 8 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5, ease: [0.19, 1, 0.22, 1] }}
          className={`t-eyebrow ${d.gap}`}
        >
          {eyebrow}
        </motion.p>
        {children}
      </div>
    </section>
  );
}
