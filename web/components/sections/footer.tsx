"use client";

import { motion, useInView } from "framer-motion";
import { useEffect, useRef, useState } from "react";

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

const TERMINAL_LINES: { prompt: string; text: string; accent?: boolean }[] = [
  { prompt: "$", text: "cargo add safe-oracle" },
  { prompt: " ", text: "  Adding safe-oracle v0.2.0 to dependencies" },
  { prompt: " ", text: "  Verifying guardrails... 5 / 5 active" },
  { prompt: " ", text: "  Tests: 310 passed; 0 failed" },
  { prompt: " ", text: "  ✓ Ready to defend", accent: true },
];

/**
 * Closing slide — final page of the deck.
 *
 * `.page-foot` gives a flush leading edge (no rounded lip) so the deck
 * resolves into one continuous closing surface as it covers the last
 * content slide, instead of an orphaned strip.
 *
 * Layout (Pass 5A): three vertically-distributed blocks.
 *   1. Huge wordmark + tagline (anchor)
 *   2. Animated terminal (cargo add demo, runs ONCE on viewport entry)
 *   3. Links + copyright + closing slogan ('Verify the integrator.'
 *      — mirrors the Hero headline, bookending the deck)
 *
 * Performance: the terminal animation is gated by `useInView({ once:
 * true })` and consists of a setTimeout chain that exhausts after
 * ~3 seconds. No infinite loops, no continuous CSS animations after
 * typing completes, no `will-change` (Pass 4B GPU-memory hygiene).
 * Total ongoing cost after the first viewport entry: zero.
 */

function AnimatedTerminal() {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { once: true, margin: "-50px" });
  const [visibleLines, setVisibleLines] = useState(0);

  useEffect(() => {
    if (!inView) return;
    const timers: ReturnType<typeof setTimeout>[] = [];
    TERMINAL_LINES.forEach((_, i) => {
      const delay = i === 0 ? 350 : 350 + i * 520;
      timers.push(
        setTimeout(() => {
          setVisibleLines(i + 1);
        }, delay)
      );
    });
    return () => {
      for (const t of timers) clearTimeout(t);
    };
  }, [inView]);

  return (
    <div
      ref={ref}
      className="mx-auto w-full max-w-xl rounded-lg border border-border/60 bg-surface/40 p-5 font-mono text-sm shadow-lg shadow-black/20"
    >
      <div className="mb-3 flex gap-1.5">
        <span className="h-2.5 w-2.5 rounded-full bg-text-dim/40" />
        <span className="h-2.5 w-2.5 rounded-full bg-text-dim/40" />
        <span className="h-2.5 w-2.5 rounded-full bg-text-dim/40" />
      </div>
      {TERMINAL_LINES.map((line, i) => {
        const visible = i < visibleLines;
        const isLastTyped = i === visibleLines - 1;
        const isAllDone = visibleLines >= TERMINAL_LINES.length;
        return (
          <div
            key={i}
            className="flex items-center gap-2 leading-relaxed"
            style={{
              minHeight: "1.6em",
              opacity: visible ? 1 : 0,
              transition: "opacity 220ms ease-out",
            }}
          >
            <span className="select-none text-text-muted">{line.prompt}</span>
            <span className={line.accent ? "text-accent" : "text-text"}>
              {line.text}
            </span>
            {visible && isLastTyped && !isAllDone && (
              <span className="ml-0.5 inline-block h-3.5 w-1.5 animate-pulse bg-accent/80" />
            )}
          </div>
        );
      })}
    </div>
  );
}

export function Footer() {
  return (
    <footer className="page-foot relative h-full bg-[var(--color-background)]">
      <div className="screen-min mx-auto flex w-full max-w-5xl flex-col px-6 py-[clamp(2.5rem,6vh,4.5rem)]">
        {/* Block 1 — huge wordmark + tagline */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.7, ease: EASE }}
        >
          <div
            className="font-mono font-medium tracking-tight text-text"
            style={{ fontSize: "clamp(2.5rem, 8vw, 5rem)", lineHeight: 1.02 }}
          >
            safe-oracle
          </div>
          <p className="mt-4 max-w-md text-base leading-relaxed text-text-muted">
            Drop-in oracle protection for Stellar Soroban.
            <br />
            Open source. Apache-2.0.
          </p>
        </motion.div>

        {/* Block 2 — animated terminal */}
        <div className="mt-8 flex flex-1 items-center justify-center md:mt-10">
          <AnimatedTerminal />
        </div>

        {/* Block 3 — links + copyright + closing slogan */}
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true, margin: "-30px" }}
          transition={{ delay: 0.3, duration: 0.6 }}
          className="mt-8 flex flex-col gap-6 border-t border-border pt-6 md:mt-10"
        >
          <div className="flex flex-wrap gap-x-8 gap-y-3 font-mono text-sm">
            <a
              href="https://github.com/Sahveli01/soroban-oracle-safety"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              GitHub ↗
            </a>
            <a
              href="https://github.com/Sahveli01/soroban-oracle-safety/blob/main/DEPLOYMENT.md"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              Docs ↗
            </a>
            <a
              href="https://stellar.org/soroban"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              Stellar Soroban ↗
            </a>
          </div>

          <div className="flex flex-col gap-2 font-mono text-xs text-text-muted sm:flex-row sm:items-center sm:justify-between">
            <span>Built for Stellar Soroban · © 2026</span>
            <span className="text-text-muted">
              Trust the oracle.{" "}
              <span className="text-accent">Verify the integrator.</span>
            </span>
          </div>
        </motion.div>
      </div>
    </footer>
  );
}
