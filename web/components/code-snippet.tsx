"use client";

import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";

/**
 * Inline copy-to-clipboard command pill.
 *
 * Premium copy feedback: the pill pulses an accent ring, and the icon
 * cross-fades to a check whose stroke draws itself. The whole pill is
 * the button and shows a pointer cursor (it's all clickable).
 */
export function CodeSnippet({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      // Browsers without clipboard permission silently no-op.
    }
  };

  return (
    <motion.button
      type="button"
      onClick={copy}
      aria-label="Copy command to clipboard"
      whileTap={{ scale: 0.97 }}
      animate={{
        boxShadow: copied
          ? "0 0 0 1px var(--color-accent), 0 0 28px -6px var(--color-accent-glow)"
          : "0 0 0 0 rgba(0,0,0,0)",
      }}
      transition={{ duration: 0.25 }}
      className="code-block flex cursor-pointer items-center gap-3 px-5 py-3"
    >
      <span className="text-[var(--color-text-dim)]">$</span>
      <span className="text-[var(--color-text)]">{code}</span>
      <span className="relative ml-2 flex h-4 w-[64px] items-center justify-end">
        <AnimatePresence mode="wait" initial={false}>
          {copied ? (
            <motion.span
              key="done"
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -4 }}
              transition={{ duration: 0.2 }}
              className="flex items-center gap-1 text-sm text-[var(--color-accent)]"
            >
              <svg
                width="14"
                height="14"
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
              className="text-[var(--color-text-dim)]"
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <rect x="9" y="9" width="13" height="13" rx="2" />
                <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
              </svg>
            </motion.span>
          )}
        </AnimatePresence>
      </span>
    </motion.button>
  );
}
