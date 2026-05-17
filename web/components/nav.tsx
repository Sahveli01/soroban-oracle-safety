"use client";

import { motion } from "framer-motion";
import Link from "next/link";
import { useLenis } from "lenis/react";
import { useState } from "react";

/**
 * Sticky pillow navigation.
 *
 * Scroll-aware: condenses + darkens after the first scroll
 * (`data-scrolled`), and highlights whichever linked section is
 * currently in view (wayfinding on a long stacked page). In-page links
 * glide via Lenis `scrollTo` (ease-out-quart, ~1.4s) — the page-turn
 * happens on the user's click, never as an automatic snap.
 */
const LINKS = [
  { id: "how-it-works", label: "How it works" },
  { id: "live", label: "Live" },
];

export function Nav() {
  const [scrolled, setScrolled] = useState(false);
  const [active, setActive] = useState<string | null>(null);

  // One hook call gives both the scroll callback and the instance.
  const lenis = useLenis((instance) => {
    const scroll = instance.scroll;
    setScrolled(scroll > 32);

    // Active = the last linked section whose top has passed the
    // viewport's upper third.
    const marker = window.innerHeight * 0.34;
    let current: string | null = null;
    for (const { id } of LINKS) {
      const el = document.getElementById(id);
      if (el && el.getBoundingClientRect().top <= marker) current = id;
    }
    setActive(current);
  });

  const scrollTo = (id: string) => {
    const el = document.getElementById(id);
    if (!el) return;
    if (lenis) {
      lenis.scrollTo(el, {
        duration: 1.4,
        easing: (t: number) => 1 - Math.pow(1 - t, 4), // ease-out-quart
      });
    } else {
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
      <div
        data-scrolled={scrolled}
        className={`pillow-nav flex items-center gap-8 ${
          scrolled ? "px-5 py-2.5" : "px-6 py-3"
        }`}
      >
        <Link
          href="/"
          className="font-mono text-sm font-medium text-[var(--color-text)]"
        >
          safe-oracle
        </Link>

        <div className="hidden items-center gap-6 md:flex">
          {LINKS.map(({ id, label }) => (
            <button
              key={id}
              onClick={() => scrollTo(id)}
              className={`cursor-pointer text-sm transition-colors ${
                active === id
                  ? "text-[var(--color-accent)]"
                  : "text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
              }`}
            >
              {label}
            </button>
          ))}
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
