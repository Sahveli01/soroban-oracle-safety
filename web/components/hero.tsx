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
 * Hero section.
 *
 * Animation timeline (from STACK.md):
 *   0.4s  — top slogan fade-in
 *   0.8s  — headline word-by-word (80ms stagger), "oracle." in accent
 *   1.6s  — subline blur-in
 *   2.0s  — code snippet slide-up
 *   2.3s  — CTAs slide-up
 *   on-scroll — stats counter spring, marquee continuous
 */
export function Hero() {
  return (
    <section id="hero" className="relative flex min-h-screen flex-col items-center px-6 pt-32 pb-24">
      {/* Top slogan */}
      <motion.p
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.4, duration: 0.6 }}
        className="mb-12 font-mono text-sm uppercase tracking-widest text-[var(--color-text-muted)]"
      >
        {TOP_SLOGAN}
      </motion.p>

      {/* Big headline — word-by-word stagger */}
      <h1 className="text-center text-5xl font-medium leading-[1.05] tracking-tight sm:text-6xl md:text-7xl lg:text-8xl">
        <span className="block">
          {HEADLINE_LINE_1.map((word, i) => (
            <motion.span
              key={`l1-${i}`}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{
                delay: 0.8 + i * 0.08,
                duration: 0.6,
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
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{
                delay: 1.1 + i * 0.08,
                duration: 0.6,
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
        initial={{ opacity: 0, filter: "blur(8px)" }}
        animate={{ opacity: 1, filter: "blur(0px)" }}
        transition={{ delay: 1.6, duration: 0.8 }}
        className="mt-10 max-w-2xl whitespace-pre-line text-center text-lg leading-relaxed text-[var(--color-text-muted)]"
      >
        {SUBLINE}
      </motion.p>

      {/* Code snippet */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 2.0, duration: 0.6 }}
        className="mt-12"
      >
        <CodeSnippet code="cargo add safe-oracle" />
      </motion.div>

      {/* CTAs */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 2.3, duration: 0.6 }}
        className="mt-8 flex items-center gap-4"
      >
        <a
          href="https://github.com/Sahveli01/soroban-oracle-safety"
          target="_blank"
          rel="noopener noreferrer"
          className="btn-primary"
        >
          View on GitHub →
        </a>
        <a href="#how-it-works" className="btn-secondary">
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

      {/* Subtle scroll cue — invites scrolling without hijacking it */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 3.5, duration: 1 }}
        className="absolute bottom-8 left-1/2 -translate-x-1/2"
      >
        <motion.div
          animate={{ y: [0, 8, 0] }}
          transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
          className="flex flex-col items-center gap-2 font-mono text-xs uppercase tracking-widest text-[var(--color-text-dim)]"
        >
          <span>scroll</span>
          <span>↓</span>
        </motion.div>
      </motion.div>
    </section>
  );
}
