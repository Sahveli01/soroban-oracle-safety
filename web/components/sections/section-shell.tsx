"use client";

import { motion, useScroll, useTransform } from "framer-motion";
import { useRef, type ReactNode } from "react";

interface SectionShellProps {
  id: string;
  eyebrow: string;
  children: ReactNode;
}

/**
 * Common section wrapper.
 *
 * - `min-h-screen`: every section is at least a full viewport, so it
 *   reads as a discrete "page" — pairs with the Lenis Snap controller
 *   (see `lenis-provider.tsx`) that snaps scroll to each `section[id]`.
 * - Subtle parallax + edge-fade as the section travels through the
 *   viewport (Noether-style page-turn feel). Amplitude is intentionally
 *   small (±20px / 0.4 min opacity) — a micro-shift, not a gimmick.
 * - The eyebrow reveal and every child `whileInView` animation are
 *   preserved unchanged; this wrapper only adds an outer transform.
 */
export function SectionShell({ id, eyebrow, children }: SectionShellProps) {
  const ref = useRef<HTMLElement>(null);

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ["start end", "end start"],
  });

  // Content drifts ±20px as the section scrolls through the viewport.
  const y = useTransform(scrollYProgress, [0, 1], [20, -20]);

  // Section "comes alive" near viewport center, recedes at the edges.
  const opacity = useTransform(
    scrollYProgress,
    [0, 0.2, 0.8, 1],
    [0.4, 1, 1, 0.4]
  );

  return (
    <section
      ref={ref}
      id={id}
      className="relative min-h-screen px-6 py-20 md:py-32 lg:py-48"
    >
      <motion.div style={{ y, opacity }} className="mx-auto max-w-5xl">
        <motion.p
          initial={{ opacity: 0, y: 10 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-100px" }}
          transition={{ duration: 0.5, ease: [0.19, 1, 0.22, 1] }}
          className="mb-8 font-mono text-xs uppercase tracking-[0.2em] text-text-muted"
        >
          {eyebrow}
        </motion.p>
        {children}
      </motion.div>
    </section>
  );
}
