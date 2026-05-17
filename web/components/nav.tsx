"use client";

import { motion } from "framer-motion";
import Link from "next/link";
import { useLenis } from "lenis/react";

/**
 * Sticky pillow navigation.
 *
 * Centered capsule, backdrop blur over body content. Mounts with a
 * subtle slide-down. The right-side ● indicator is a real status dot
 * — testnet contracts are live (Phase 7 closure).
 *
 * In-page links use Lenis `scrollTo` for a smooth, user-initiated
 * glide to the target section (ease-out-quart, ~1.4s) — the page-turn
 * happens on the user's click, never as an automatic snap.
 */
export function Nav() {
  const lenis = useLenis();

  const scrollTo = (target: string) => {
    const el = document.querySelector(target);
    if (!el) return;
    if (lenis) {
      lenis.scrollTo(el as HTMLElement, {
        duration: 1.4,
        easing: (t: number) => 1 - Math.pow(1 - t, 4), // ease-out-quart
      });
    } else {
      // Lenis not ready yet — fall back to native smooth scroll.
      el.scrollIntoView({ behavior: "smooth" });
    }
  };

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
          <button
            onClick={() => scrollTo("#how-it-works")}
            className="cursor-pointer text-sm text-[var(--color-text-muted)] transition-colors hover:text-[var(--color-text)]"
          >
            How it works
          </button>
          <button
            onClick={() => scrollTo("#live")}
            className="cursor-pointer text-sm text-[var(--color-text-muted)] transition-colors hover:text-[var(--color-text)]"
          >
            Live
          </button>
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
