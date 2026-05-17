"use client";

import { motion } from "framer-motion";
import Link from "next/link";
import { useEffect, useState } from "react";
import { scrollToSection } from "@/components/section-snap";

/**
 * Sticky pillow navigation.
 *
 * Scroll-aware: condenses + darkens after the first scroll
 * (`data-scrolled`) and highlights whichever linked section is in view.
 *
 * Sections are `position: sticky` — a pinned panel reports rect.top ≈ 0,
 * so wayfinding uses the element's true *flow* position (`docTop`),
 * never its rect. Link clicks drive the SectionSnap engine via
 * `scrollToSection`, so a nav jump uses the exact same eased,
 * page-aligned transition as a wheel/keyboard gesture.
 */
const LINKS = [
  { id: "how-it-works", label: "How it works" },
  { id: "live", label: "Live" },
];

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

  useEffect(() => {
    let raf = 0;
    const update = () => {
      const y = window.scrollY;
      setScrolled(y > 32);
      const marker = y + window.innerHeight * 0.34;
      let current: string | null = null;
      for (const { id } of LINKS) {
        const el = document.getElementById(id);
        if (el && docTop(el) <= marker) current = id;
      }
      setActive(current);
    };
    const onScroll = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(update);
    };
    update();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll, { passive: true });
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
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
              onClick={() => scrollToSection(id)}
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
