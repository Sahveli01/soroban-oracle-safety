"use client";

import { motion } from "framer-motion";
import { useRef, type ReactNode } from "react";

import { useRecede } from "../use-recede";

interface SectionShellProps {
  id: string;
  eyebrow: string;
  children: ReactNode;
}

/**
 * Stacked section panel — the "notebook page-turn".
 *
 * Every section is a `sticky top-0` panel with an opaque background and
 * a rounded, soft-shadowed leading edge (`.page-panel`). Because the
 * panels are consecutive sticky siblings, the next one slides up and
 * covers this one; meanwhile this panel's content recedes (scale/dim/
 * blur via `useRecede`) so it reads as a page tucking underneath.
 *
 * The motion is purely scroll-linked — there is no snap and nothing
 * overrides the user's scroll. Eyebrow reveal + all child
 * `whileInView` animations are preserved.
 */
export function SectionShell({ id, eyebrow, children }: SectionShellProps) {
  const ref = useRef<HTMLElement>(null);
  const { scale, opacity, filter } = useRecede(ref);

  return (
    <section
      ref={ref}
      id={id}
      className="page-panel sticky top-0 flex min-h-screen items-center px-6 py-24 md:py-28"
    >
      <motion.div
        style={{ scale, opacity, filter }}
        className="mx-auto w-full max-w-5xl origin-top will-change-transform"
      >
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
