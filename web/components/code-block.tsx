"use client";

import { useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";

/**
 * Multiline syntax-tinted code block with the same copy affordance as
 * the inline CodeSnippet pill (accent ring pulse + drawn check). Reuses
 * the `.code-block` surface so it sits in the existing design language.
 *
 * The Hero slogan promises "Eight lines"; this renders the actual
 * eight-line integration as the proof of that claim.
 */

const KW = "text-[var(--color-accent)]";
const TY = "text-text";
const FN = "text-text";
const PUNCT = "text-text-muted";
const DIM = "text-[var(--color-text-dim)]";

// Exactly eight lines — the payoff for "Eight lines. Five guards."
const RAW = `use safe_oracle::{lastprice, SafeOracleConfig};

let price = lastprice(
    &env, &asset,
    &reflector, &registry,
    &SafeOracleConfig::default(),
)?;
// 5 guards validated before this line.`;

const LINES: ReactNode[] = [
  <>
    <span className={KW}>use</span>{" "}
    <span className={TY}>safe_oracle</span>
    <span className={PUNCT}>::{"{"}</span>
    <span className={FN}>lastprice</span>
    <span className={PUNCT}>, </span>
    <span className={TY}>SafeOracleConfig</span>
    <span className={PUNCT}>{"}"};</span>
  </>,
  <>&nbsp;</>,
  <>
    <span className={KW}>let</span> <span className={TY}>price</span>{" "}
    <span className={PUNCT}>=</span> <span className={FN}>lastprice</span>
    <span className={PUNCT}>(</span>
  </>,
  <>
    <span className={PUNCT}>&nbsp;&nbsp;&nbsp;&nbsp;&amp;</span>
    <span className={TY}>env</span>
    <span className={PUNCT}>, &amp;</span>
    <span className={TY}>asset</span>
    <span className={PUNCT}>,</span>
  </>,
  <>
    <span className={PUNCT}>&nbsp;&nbsp;&nbsp;&nbsp;&amp;</span>
    <span className={TY}>reflector</span>
    <span className={PUNCT}>, &amp;</span>
    <span className={TY}>registry</span>
    <span className={PUNCT}>,</span>
  </>,
  <>
    <span className={PUNCT}>&nbsp;&nbsp;&nbsp;&nbsp;&amp;</span>
    <span className={TY}>SafeOracleConfig</span>
    <span className={PUNCT}>::</span>
    <span className={FN}>default</span>
    <span className={PUNCT}>(),</span>
  </>,
  <>
    <span className={PUNCT}>)?;</span>
  </>,
  <>
    <span className={DIM}>// 5 guards validated before this line.</span>
  </>,
];

export function CodeBlock() {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(RAW);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      // Browsers without clipboard permission silently no-op.
    }
  };

  return (
    <motion.div
      animate={{
        boxShadow: copied
          ? "0 0 0 1px var(--color-accent), 0 0 28px -6px var(--color-accent-glow)"
          : "0 0 0 0 rgba(0,0,0,0)",
      }}
      transition={{ duration: 0.25 }}
      className="code-block w-full max-w-xl overflow-hidden text-left"
    >
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <span className="font-mono text-[var(--color-text-dim)] text-xs">
          src/lib.rs
        </span>
        <button
          type="button"
          onClick={copy}
          aria-label="Copy code to clipboard"
          className="flex cursor-pointer items-center gap-1.5 font-mono text-xs text-[var(--color-text-dim)] transition-colors hover:text-[var(--color-text)]"
        >
          <AnimatePresence mode="wait" initial={false}>
            {copied ? (
              <motion.span
                key="done"
                initial={{ opacity: 0, y: 3 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -3 }}
                transition={{ duration: 0.2 }}
                className="flex items-center gap-1 text-[var(--color-accent)]"
              >
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="3"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <motion.path
                    d="M20 6 9 17l-5-5"
                    initial={{ pathLength: 0 }}
                    animate={{ pathLength: 1 }}
                    transition={{ duration: 0.35, ease: "easeOut" }}
                  />
                </svg>
                copied
              </motion.span>
            ) : (
              <motion.span
                key="copy"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
              >
                copy
              </motion.span>
            )}
          </AnimatePresence>
        </button>
      </div>
      <pre className="overflow-x-auto px-4 py-4 font-mono text-sm leading-relaxed">
        <code>
          {LINES.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </code>
      </pre>
    </motion.div>
  );
}
