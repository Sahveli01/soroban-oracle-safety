"use client";

import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import { deckGoTo } from "@/components/deck";

/**
 * Pillow navigation.
 *
 * The deck owns navigation now (integer slide index, no scroll). The
 * nav listens to the `deck:change` event for the active slide and
 * subtle condensed state; a link click calls `deckGoTo(id)`, which
 * animates to that slide with the exact same transition as a wheel
 * gesture. No scroll math anywhere ⇒ nav-back simply cannot fail.
 */
const LINKS = [
  { id: "how-it-works", label: "How it works" },
  { id: "live", label: "Live" },
];

interface DeckChange {
  index: number;
  id?: string;
  total: number;
}

export function Nav() {
  const [scrolled, setScrolled] = useState(false);
  const [activeId, setActiveId] = useState<string | null>(null);

  useEffect(() => {
    const onChange = (e: Event) => {
      const d = (e as CustomEvent<DeckChange>).detail;
      setScrolled(d.index > 0);
      setActiveId(d.id ?? null);
    };
    window.addEventListener("deck:change", onChange);
    return () => window.removeEventListener("deck:change", onChange);
  }, []);

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
        <button
          onClick={() => deckGoTo("hero")}
          className="cursor-pointer font-mono text-sm font-medium text-[var(--color-text)]"
        >
          safe-oracle
        </button>

        <div className="hidden items-center gap-6 md:flex">
          {LINKS.map(({ id, label }) => (
            <button
              key={id}
              onClick={() => deckGoTo(id)}
              className={`cursor-pointer text-sm transition-colors ${
                activeId === id
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
      </div>
    </motion.nav>
  );
}
