"use client";

import { motion } from "framer-motion";
import type { ReactNode } from "react";

interface SectionShellProps {
  id: string;
  eyebrow: string;
  children: ReactNode;
}

/**
 * Common section wrapper with eyebrow label + reveal animation.
 * Each main content section uses this for consistent spacing + tone.
 */
export function SectionShell({ id, eyebrow, children }: SectionShellProps) {
  return (
    <section id={id} className="relative px-6 py-32 md:py-48">
      <div className="mx-auto max-w-5xl">
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
      </div>
    </section>
  );
}
