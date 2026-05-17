"use client";

import { motion } from "framer-motion";
import { useRef } from "react";
import { CodeSnippet } from "./code-snippet";
import { StatsRow } from "./stats-row";
import { Marquee } from "./marquee";
import { useRecede } from "./use-recede";

const TOP_SLOGAN = "Eight lines. Five guards. Zero exploits.";

const HEADLINE_LINE_1 = ["Trust", "the", "oracle."];
const HEADLINE_LINE_2 = ["Verify", "the", "integrator."];

const SUBLINE = `Five mathematically-verified guardrails between
your protocol and the next oracle manipulation attack.`;

const EASE_OUT_EXPO: [number, number, number, number] = [0.19, 1, 0.22, 1];

/**
 * Hero — the first stacked panel.
 *
 * Front-loaded timeline (everything resolves under ~1.4s, premium sites
 * never make you wait): slogan .15s → headline word stagger .3s →
 * subline .7s → command .95s → CTAs 1.1s. A soft accent aurora sits
 * behind the headline; a drawn-line indicator invites the scroll.
 * Recedes under the next panel like every other section.
 */
export function Hero() {
  const ref = useRef<HTMLElement>(null);
  const { scale, opacity, filter } = useRecede(ref);

  return (
    <section
      ref={ref}
      id="hero"
      className="page-panel sticky top-0 flex min-h-screen flex-col items-center justify-center overflow-hidden px-6 py-28"
    >
      {/* Focal aurora — soft accent bloom behind the headline */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute left-1/2 top-[38%] h-[34rem] w-[34rem] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-60 blur-[120px]"
        style={{
          background:
            "radial-gradient(circle, var(--color-accent-glow), transparent 70%)",
        }}
      />

      <motion.div
        style={{ scale, opacity, filter }}
        className="relative flex w-full origin-top flex-col items-center will-change-transform"
      >
        {/* Top slogan */}
        <motion.p
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mb-10 font-mono text-sm uppercase tracking-[0.25em] text-[var(--color-text-muted)]"
        >
          {TOP_SLOGAN}
        </motion.p>

        {/* Big headline — word-by-word stagger */}
        <h1 className="text-balance text-center text-5xl font-semibold leading-[1.04] tracking-[-0.03em] sm:text-6xl md:text-7xl lg:text-8xl">
          <span className="block">
            {HEADLINE_LINE_1.map((word, i) => (
              <motion.span
                key={`l1-${i}`}
                initial={{ opacity: 0, y: 24 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  delay: 0.3 + i * 0.07,
                  duration: 0.7,
                  ease: EASE_OUT_EXPO,
                }}
                className={`inline-block ${
                  word === "oracle." ? "text-[var(--color-accent)]" : ""
                }`}
              >
                {word}
                {i < HEADLINE_LINE_1.length - 1 && " "}
              </motion.span>
            ))}
          </span>
          <span className="block">
            {HEADLINE_LINE_2.map((word, i) => (
              <motion.span
                key={`l2-${i}`}
                initial={{ opacity: 0, y: 24 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  delay: 0.5 + i * 0.07,
                  duration: 0.7,
                  ease: EASE_OUT_EXPO,
                }}
                className="inline-block"
              >
                {word}
                {i < HEADLINE_LINE_2.length - 1 && " "}
              </motion.span>
            ))}
          </span>
        </h1>

        {/* Subline */}
        <motion.p
          initial={{ opacity: 0, filter: "blur(8px)" }}
          animate={{ opacity: 1, filter: "blur(0px)" }}
          transition={{ delay: 0.7, duration: 0.8 }}
          className="mt-10 max-w-2xl whitespace-pre-line text-center text-lg leading-relaxed text-[var(--color-text-muted)]"
        >
          {SUBLINE}
        </motion.p>

        {/* Code snippet */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.95, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-12"
        >
          <CodeSnippet code="cargo add safe-oracle" />
        </motion.div>

        {/* CTAs */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 1.1, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-8 flex flex-wrap items-center justify-center gap-4"
        >
          <a
            href="https://github.com/Sahveli01/soroban-oracle-safety"
            target="_blank"
            rel="noopener noreferrer"
            className="btn-primary"
          >
            View on GitHub →
          </a>
          <a
            href="https://github.com/Sahveli01/soroban-oracle-safety/blob/main/DEPLOYMENT.md"
            target="_blank"
            rel="noopener noreferrer"
            className="btn-secondary"
          >
            Read the docs
          </a>
        </motion.div>

        {/* Stats row */}
        <div className="mt-24 w-full max-w-4xl">
          <StatsRow />
        </div>

        {/* Marquee */}
        <div className="mt-24 w-full">
          <Marquee />
        </div>
      </motion.div>

      {/* Scroll indicator — a drawn line that pulses downward */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1.6, duration: 1 }}
        className="absolute bottom-9 left-1/2 flex -translate-x-1/2 flex-col items-center gap-3"
      >
        <span className="font-mono text-[10px] uppercase tracking-[0.3em] text-[var(--color-text-dim)]">
          Scroll
        </span>
        <span className="relative h-10 w-px overflow-hidden bg-[var(--color-border-strong)]">
          <motion.span
            className="absolute inset-x-0 top-0 h-1/2 bg-[var(--color-accent)]"
            animate={{ y: ["-100%", "200%"] }}
            transition={{
              duration: 1.8,
              repeat: Infinity,
              ease: "easeInOut",
            }}
          />
        </span>
      </motion.div>
    </section>
  );
}
