"use client";

import { motion } from "framer-motion";
import Link from "next/link";

/**
 * Sticky pillow navigation.
 *
 * Centered capsule, backdrop blur over body content. Mounts with a
 * subtle slide-down. The right-side ● indicator is a real status dot
 * — testnet contracts are live (Phase 7 closure).
 */
export function Nav() {
  return (
    <motion.nav
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.19, 1, 0.22, 1] }}
      className="fixed left-1/2 top-6 z-50 -translate-x-1/2"
    >
      <div className="pillow-nav flex items-center gap-8 px-6 py-3">
        <Link
          href="/"
          className="font-mono text-sm font-medium text-[var(--color-text)]"
        >
          safe-oracle
        </Link>

        <div className="hidden items-center gap-6 md:flex">
          <a
            href="#how-it-works"
            className="text-sm text-[var(--color-text-muted)] transition-colors hover:text-[var(--color-text)]"
          >
            How it works
          </a>
          <a
            href="#live"
            className="text-sm text-[var(--color-text-muted)] transition-colors hover:text-[var(--color-text)]"
          >
            Live
          </a>
          <a
            href="https://github.com/Sahveli01/soroban-oracle-safety"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-[var(--color-text-muted)] transition-colors hover:text-[var(--color-text)]"
          >
            GitHub ↗
          </a>
        </div>

        <div className="flex items-center gap-2">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--color-accent)] opacity-75" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-[var(--color-accent)]" />
          </span>
          <span className="font-mono text-xs text-[var(--color-text-muted)]">
            testnet live
          </span>
        </div>
      </div>
    </motion.nav>
  );
}
