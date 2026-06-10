"use client";

import { motion, useInView } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { EASE_OUT_EXPO } from "@/lib/easings";

const TERMINAL_LINES: { prompt: string; text: string; accent?: boolean }[] = [
  { prompt: "$", text: "cargo add safe-oracle" },
  { prompt: " ", text: "  Adding safe-oracle v0.4.0 to dependencies" },
  { prompt: " ", text: "  Layer 1 guards active · Layer 2 opt-in" },
  { prompt: " ", text: "  Tests: 321 passed; 0 failed" },
  { prompt: " ", text: "  ✓ Ready to defend", accent: true },
];

// Pass 8 typewriter timings. CHAR_DELAY ≈ 45 chars/sec — fast enough
// to feel real, slow enough to read each character land. LINE_GAP is
// the pause after a line completes before the next line begins typing.
const CHAR_DELAY_MS = 22;
const LINE_GAP_MS = 180;

/**
 * Closing slide — final page of the deck.
 *
 * `.page-foot` gives a flush leading edge (no rounded lip) so the deck
 * resolves into one continuous closing surface as it covers the last
 * content slide, instead of an orphaned strip.
 *
 * Layout (Pass 5A): three vertically-distributed blocks.
 *   1. Huge wordmark + tagline (anchor)
 *   2. Typewriter terminal (cargo add demo, RE-PLAYS on every
 *      viewport entry — Pass 8 user request)
 *   3. Links + copyright + closing slogan ('Verify the integrator.'
 *      — mirrors the Hero headline, bookending the deck)
 *
 * Performance: the typewriter is a single recursive setTimeout chain
 * (NOT setInterval) with a `cancelled` flag + clearTimeout on cleanup.
 * Rapid back-and-forth scroll between Footer and the previous slide
 * cancels in-flight work cleanly: the next entry restarts from zero
 * via an animationKey bump. After the last character types, the
 * chain exits; ongoing cost is zero until the next viewport entry.
 * No `will-change`, no infinite CSS animation (cursor blink uses
 * Tailwind's `animate-pulse` which respects prefers-reduced-motion
 * via the globals.css override).
 */

function TypewriterTerminal() {
  const ref = useRef<HTMLDivElement>(null);
  // No `once: true` — user wants the typewriter to replay every
  // time the Footer slide enters the viewport.
  const inView = useInView(ref, { margin: "-50px" });

  // visibleChars[i] = how many characters of line i are currently typed
  const [visibleChars, setVisibleChars] = useState<number[]>(() =>
    TERMINAL_LINES.map(() => 0)
  );
  const [activeLineIdx, setActiveLineIdx] = useState(0);
  // Bumped on every fresh inView=true so the typing useEffect re-fires
  // with a known-clean slate, even if React would otherwise have
  // skipped the effect (same deps as last run).
  const [animationKey, setAnimationKey] = useState(0);

  // Reset + bump key whenever the slide enters the viewport.
  useEffect(() => {
    if (!inView) return;
    setVisibleChars(TERMINAL_LINES.map(() => 0));
    setActiveLineIdx(0);
    setAnimationKey((k) => k + 1);
  }, [inView]);

  // Driver: one recursive setTimeout that walks (lineIdx, charIdx).
  // Single timer at a time, cancelled flag + clearTimeout on unmount /
  // re-trigger keeps the chain safe under rapid slide changes.
  useEffect(() => {
    if (!inView) return;

    let cancelled = false;
    let timeoutId: ReturnType<typeof setTimeout> | null = null;

    const typeNextChar = (lineIdx: number, charIdx: number) => {
      if (cancelled) return;
      if (lineIdx >= TERMINAL_LINES.length) return;

      const line = TERMINAL_LINES[lineIdx];
      if (charIdx > line.text.length) {
        // Line just finished — short gap, then advance to next line.
        timeoutId = setTimeout(() => {
          if (cancelled) return;
          setActiveLineIdx(lineIdx + 1);
          typeNextChar(lineIdx + 1, 0);
        }, LINE_GAP_MS);
        return;
      }

      setVisibleChars((prev) => {
        const next = [...prev];
        next[lineIdx] = charIdx;
        return next;
      });

      timeoutId = setTimeout(() => {
        typeNextChar(lineIdx, charIdx + 1);
      }, CHAR_DELAY_MS);
    };

    typeNextChar(0, 0);

    return () => {
      cancelled = true;
      if (timeoutId !== null) clearTimeout(timeoutId);
    };
  }, [animationKey, inView]);

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
        const charsShown = visibleChars[i];
        const isComplete = charsShown >= line.text.length;
        const isActiveLine = i === activeLineIdx && i < TERMINAL_LINES.length;
        const displayedText = line.text.slice(0, charsShown);
        return (
          <div
            key={i}
            className="flex items-center gap-2 leading-relaxed"
            style={{ minHeight: "1.6em" }}
          >
            <span className="select-none text-text-muted">{line.prompt}</span>
            <span className={line.accent ? "text-accent" : "text-text"}>
              {displayedText}
              {isActiveLine && !isComplete && (
                <span className="ml-0.5 inline-block h-3.5 w-1.5 animate-pulse bg-accent/80 align-middle" />
              )}
            </span>
          </div>
        );
      })}
    </div>
  );
}

export function Footer() {
  return (
    <footer className="page-foot relative h-full bg-[var(--color-background)]">
      {/* Pass 7 fix: the three blocks now justify-center as a tight
          group instead of {top, flex-1 middle, bottom}, which was making
          the terminal float alone in a big middle band with empty space
          above the wordmark. Removed flex-1 on middle, added
          justify-center on the screen-min, tightened gaps. */}
      <div className="screen-min mx-auto flex w-full max-w-5xl flex-col justify-center gap-7 px-6 py-[clamp(2rem,5vh,3.5rem)]">
        {/* Block 1 — huge wordmark + tagline */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.7, ease: EASE_OUT_EXPO }}
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

        {/* Block 2 — typewriter terminal (Pass 8: replays every entry) */}
        <div className="flex justify-center">
          <TypewriterTerminal />
        </div>

        {/* Block 3 — links + copyright + closing slogan */}
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true, margin: "-30px" }}
          transition={{ delay: 0.3, duration: 0.6 }}
          className="flex flex-col gap-6 border-t border-border pt-6"
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
              href="https://github.com/Sahveli01/soroban-oracle-safety/tree/main/docs/soroban-oracle-security"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              Security Standard ↗
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
