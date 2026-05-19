"use client";

import { motion } from "framer-motion";
import { CodeSnippet } from "./code-snippet";
import { StatsRow } from "./stats-row";
import { Marquee } from "./marquee";

const TOP_SLOGAN = "Eight lines. Five guards. Zero exploits.";

const HEADLINE_LINE_1 = ["Trust", "the", "oracle."];
const HEADLINE_LINE_2 = ["Verify", "the", "integrator."];

const SUBLINE = `Five mathematically-verified guardrails between
your protocol and the next oracle manipulation attack.`;

const EASE_OUT_EXPO: [number, number, number, number] = [0.19, 1, 0.22, 1];

/**
 * Hero — the first stacked panel.
 *
 * Front-loaded timeline (resolves under ~1.4s). Clean, readable
 * typography (medium weight, tracking-tight — NOT the over-tight
 * semibold/text-balance that hurt legibility before). No scroll
 * transforms: it's a sticky panel the next section cleanly covers.
 * Content uses .screen-min (100vh→100svh) + justify-center with fluid
 * spacing, so it fills the *visible* viewport and never clips on
 * macOS Safari (where 100vh exceeds the visible area).
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
        className="pointer-events-none absolute left-1/2 top-[36%] h-[32rem] w-[32rem] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-50 blur-[130px]"
        style={{
          background:
            "radial-gradient(circle, var(--color-accent-glow), transparent 70%)",
        }}
      />

      <div className="screen-min relative mx-auto flex w-full max-w-5xl flex-col items-center justify-center px-6 py-[clamp(4rem,9vh,6rem)]">
        {/* Top slogan */}
        <motion.p
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mb-8 font-mono text-sm uppercase tracking-[0.22em] text-[#a8a8b0]"
        >
          {TOP_SLOGAN}
        </motion.p>

        {/* Big headline — word-by-word stagger */}
        <h1 className="t-h1 text-center">
          <span className="block">
            {HEADLINE_LINE_1.map((word, i) => (
              <motion.span
                key={`l1-${i}`}
                initial={{ opacity: 0, y: 22 }}
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
                {i < HEADLINE_LINE_1.length - 1 && " "}
              </motion.span>
            ))}
          </span>
          <span className="block">
            {HEADLINE_LINE_2.map((word, i) => (
              <motion.span
                key={`l2-${i}`}
                initial={{ opacity: 0, y: 22 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  delay: 0.5 + i * 0.07,
                  duration: 0.7,
                  ease: EASE_OUT_EXPO,
                }}
                className="inline-block"
              >
                {word}
                {i < HEADLINE_LINE_2.length - 1 && " "}
              </motion.span>
            ))}
          </span>
        </h1>

        {/* Subline */}
        <motion.p
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.75, duration: 0.7, ease: EASE_OUT_EXPO }}
          className="mt-7 max-w-2xl whitespace-pre-line text-center text-lg leading-relaxed text-[var(--color-text-muted)]"
        >
          {SUBLINE}
        </motion.p>

        {/* Code snippet */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.95, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-9"
        >
          <CodeSnippet code="cargo add safe-oracle" />
        </motion.div>

        {/* CTAs */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 1.1, duration: 0.6, ease: EASE_OUT_EXPO }}
          className="mt-7 flex flex-wrap items-center justify-center gap-4"
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
        <div className="mt-[clamp(2rem,5vh,3.5rem)] w-full max-w-4xl">
          <StatsRow />
        </div>

        {/* Marquee */}
        <div className="mt-[clamp(2rem,5vh,3.5rem)] w-full">
          <Marquee />
        </div>
      </div>
    </section>
  );
}
