"use client";

import { motion } from "framer-motion";
import { CodeSnippet } from "./code-snippet";
import { CodeBlock } from "./code-block";
import { StatsRow } from "./stats-row";
import { Marquee } from "./marquee";

const TOP_SLOGAN = "Eight lines. Five guards. Zero exploits.";

const HEADLINE_LINE_1 = ["Trust", "the", "oracle."];
const HEADLINE_LINE_2 = ["Verify", "the", "integrator."];

const SUBLINE =
  "Five mathematically-verified guardrails between your protocol and the next oracle manipulation attack.";

const EASE_OUT_EXPO: [number, number, number, number] = [0.19, 1, 0.22, 1];

/**
 * Hero — the first stacked panel.
 *
 * Must fit ENTIRELY in 100svh from top slogan through marquee bottom
 * at standard desktop viewports (1366×768 and up). Pass 3 fix: the
 * deck slide-by-slide model is broken the moment `.screen-min`
 * internally scrolls; before this pass, Hero's ~860px content stack
 * exceeded the ~712px content area (768 − ~56 nav) and the slide
 * silently scrolled, leaving the headline clipped and the marquee
 * reachable only by scroll-wheel.
 *
 * Strategy: compress every element. Hero-specific h1 size (smaller
 * than the body t-h1), tight CodeBlock, tight inter-element margins,
 * smaller stats text, and a thin marquee. No element removed — every
 * line of the pitch survives.
 */
export function Hero() {
  return (
    <section
      id="hero"
      className="page-panel relative h-full overflow-hidden bg-[var(--color-background)]"
    >
      {/* Focal aurora — soft accent bloom behind the headline */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute left-1/2 top-[32%] h-[28rem] w-[28rem] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-50 blur-[120px]"
        style={{
          background:
            "radial-gradient(circle, var(--color-accent-glow), transparent 70%)",
        }}
      />

      <div className="screen-min relative mx-auto flex w-full max-w-5xl flex-col items-center justify-center px-6 pb-[clamp(1rem,2vh,2rem)] pt-[clamp(3.25rem,5vh,4rem)]">
        {/* Top slogan */}
        <motion.p
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mb-3 font-mono text-xs uppercase tracking-[0.22em] text-[#a8a8b0] md:text-sm"
        >
          {TOP_SLOGAN}
        </motion.p>

        {/* Big headline — word-by-word stagger. Hero-specific size
            (`t-hero-h1`) — smaller than body `.t-h1` since Hero is the
            slide with the most competing vertical content. */}
        <h1 className="t-hero-h1 text-center">
          <span className="block">
            {HEADLINE_LINE_1.map((word, i) => (
              <motion.span
                key={`l1-${i}`}
                initial={{ opacity: 0, y: 18 }}
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
                initial={{ opacity: 0, y: 18 }}
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
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.75, duration: 0.7, ease: EASE_OUT_EXPO }}
          className="mt-4 max-w-2xl text-center text-base leading-snug text-[var(--color-text-muted)] md:text-lg"
        >
          {SUBLINE}
        </motion.p>

        {/* Eight-line proof — pays off the "Eight lines" slogan */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.9, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-5 flex w-full justify-center"
        >
          <CodeBlock />
        </motion.div>

        {/* Install command */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 1.0, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-3"
        >
          <CodeSnippet code="cargo add safe-oracle" />
        </motion.div>

        {/* CTAs */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 1.1, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-4 flex flex-wrap items-center justify-center gap-3"
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
        <div className="mt-[clamp(1rem,2vh,1.75rem)] w-full max-w-4xl">
          <StatsRow />
        </div>

        {/* Marquee */}
        <div className="mt-[clamp(0.75rem,1.5vh,1.5rem)] w-full">
          <Marquee />
        </div>
      </div>
    </section>
  );
}
