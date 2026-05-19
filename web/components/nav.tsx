"use client";

import { motion, AnimatePresence } from "framer-motion";
import { useState, useEffect } from "react";
import { deckGoTo, useDeckIndex } from "./deck";

/** Mirrors the slide order in app/page.tsx. */
const SLIDES = [
  { id: "hero", label: "safe-oracle" },
  { id: "attack", label: "Attack" },
  { id: "solution", label: "Solution" },
  { id: "how-it-works", label: "How it works" },
  { id: "architecture", label: "Architecture" },
  { id: "mechanism", label: "Mechanism" },
  { id: "infrastructure", label: "Infrastructure" },
  { id: "operator", label: "Operator" },
  { id: "live", label: "Live" },
  { id: "trust", label: "Trust" },
  { id: "footer", label: "End" },
];

export function Nav() {
  const currentIndex = useDeckIndex();
  const [isOpen, setIsOpen] = useState(false);

  // Collapse the mobile menu whenever the slide changes.
  useEffect(() => {
    setIsOpen(false);
  }, [currentIndex]);

  return (
    <motion.nav
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay: 0.2 }}
      className="fixed inset-x-0 top-0 z-50 flex items-center justify-between bg-background/40 px-6 py-4 backdrop-blur-md"
    >
      {/* Wordmark — returns to Hero */}
      <button
        onClick={() => deckGoTo("hero")}
        className="font-mono text-sm uppercase tracking-wider text-text transition-colors hover:text-accent"
      >
        safe-oracle
      </button>

      {/* Desktop: compact dot rail with per-slide tooltip */}
      <div className="hidden items-center gap-2 md:flex">
        {SLIDES.map((slide, i) => (
          <button
            key={slide.id}
            onClick={() => deckGoTo(slide.id)}
            aria-label={`Go to ${slide.label}`}
            aria-current={i === currentIndex ? "true" : undefined}
            className={`group relative h-1.5 rounded-full transition-all ${
              i === currentIndex
                ? "w-10 bg-accent"
                : "w-5 bg-border hover:bg-text-muted"
            }`}
          >
            <span className="pointer-events-none absolute left-1/2 top-full mt-2 -translate-x-1/2 whitespace-nowrap font-mono text-[10px] uppercase tracking-wider text-text-muted opacity-0 transition-opacity group-hover:opacity-100">
              {slide.label}
            </span>
          </button>
        ))}
      </div>

      {/* Mobile: hamburger */}
      <button
        onClick={() => setIsOpen((v) => !v)}
        aria-label="Toggle navigation"
        aria-expanded={isOpen}
        className="flex flex-col gap-1 p-2 md:hidden"
      >
        <span
          className={`h-px w-5 bg-text transition-transform ${
            isOpen ? "translate-y-[5px] rotate-45" : ""
          }`}
        />
        <span
          className={`h-px w-5 bg-text transition-opacity ${
            isOpen ? "opacity-0" : ""
          }`}
        />
        <span
          className={`h-px w-5 bg-text transition-transform ${
            isOpen ? "-translate-y-[5px] -rotate-45" : ""
          }`}
        />
      </button>

      {/* Mobile menu */}
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2 }}
            className="absolute inset-x-0 top-full border-b border-border bg-background md:hidden"
          >
            <div className="flex flex-col gap-3 px-6 py-4">
              {SLIDES.map((slide, i) => (
                <button
                  key={slide.id}
                  onClick={() => {
                    deckGoTo(slide.id);
                    setIsOpen(false);
                  }}
                  className={`text-left font-mono text-sm uppercase tracking-wider transition-colors ${
                    i === currentIndex
                      ? "text-accent"
                      : "text-text-muted hover:text-text"
                  }`}
                >
                  {slide.label}
                </button>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.nav>
  );
}
