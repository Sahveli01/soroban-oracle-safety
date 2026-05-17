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
 * currently in view.
 *
 * Sections are `position: sticky` — their `getBoundingClientRect()` is
 * distorted (a pinned panel reports top ≈ 0), so both wayfinding and
 * in-page jumps must use the element's true *flow* position
 * (`docTop`: summed offsetTop up the offsetParent chain), never its
 * rect. This is why the "How it works" link used to fail from lower in
 * the page — it resolved against a pinned rect and went nowhere.
 */
const LINKS = [
  { id: "how-it-works", label: "How it works" },
  { id: "live", label: "Live" },
];

/** True flow position of a sticky element, unaffected by pinning. */
function docTop(el: HTMLElement): number {
  let y = 0;
  let n: HTMLElement | null = el;
  while (n) {
    y += n.offsetTop;
    n = n.offsetParent as HTMLElement | null;
  }
  return y;
}

export function Nav() {
  const [scrolled, setScrolled] = useState(false);
  const [active, setActive] = useState<string | null>(null);

  // One hook call gives both the scroll callback and the instance.
  const lenis = useLenis((instance) => {
    const scroll = instance.scroll;
    setScrolled(scroll > 32);

    // Active = the last linked section whose flow-top has passed the
    // viewport's upper third. Uses docTop (not rect) so sticky pinning
    // doesn't make every section read as "already passed".
    const marker = scroll + window.innerHeight * 0.34;
    let current: string | null = null;
    for (const { id } of LINKS) {
      const el = document.getElementById(id);
      if (el && docTop(el) <= marker) current = id;
    }
    setActive(current);
  });

  const scrollTo = (id: string) => {
    const el = document.getElementById(id);
    if (!el) return;
    const target = docTop(el);
    if (lenis) {
      lenis.scrollTo(target, {
        duration: 1.1,
        easing: (t: number) => 1 - Math.pow(1 - t, 4), // ease-out-quart
      });
    } else {
      window.scrollTo({ top: target, behavior: "smooth" });
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
      </div>
    </motion.nav>
  );
}
