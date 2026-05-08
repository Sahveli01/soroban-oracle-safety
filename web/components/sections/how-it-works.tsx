"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const STEPS = [
  {
    num: "01",
    glyph: "◉",
    title: "Reflector Call",
    body: "Your contract calls safe_oracle::lastprice() instead of reflector directly.",
    sub: [],
  },
  {
    num: "02",
    glyph: "◈",
    title: "Layer 1 Checks",
    body: "Oracle-side validation against price feed mechanics.",
    sub: [
      "Deviation — sudden price changes blocked",
      "Staleness — outdated feeds rejected",
      "Cross-Source — secondary oracle disagreement caught",
    ],
  },
  {
    num: "03",
    glyph: "●",
    title: "Layer 2 Checks",
    body: "Market microstructure validation against on-chain liquidity reality.",
    sub: [
      "Liquidity — SDEX 30-minute volume threshold",
      "Thin Sampling — unique trader count",
    ],
  },
  {
    num: "04",
    glyph: "⚙",
    title: "Circuit Breaker",
    body: "Auto-halt after first violation. Governance manual override available.",
    sub: [],
  },
  {
    num: "05",
    glyph: "▲",
    title: "Result",
    body: "Validated price returned. Or Err with specific violation type.",
    sub: [],
  },
];

export function HowItWorks() {
  return (
    <SectionShell id="how-it-works" eyebrow="How It Works">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: [0.19, 1, 0.22, 1] }}
        className="text-5xl font-medium leading-[1.1] tracking-tight md:text-6xl"
      >
        Five steps.
        <br />
        <span className="text-accent">One result.</span>
      </motion.h2>

      <div className="mt-20 space-y-12">
        {STEPS.map((step, i) => (
          <motion.div
            key={step.num}
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{
              delay: i * 0.1,
              duration: 0.6,
              ease: [0.19, 1, 0.22, 1],
            }}
            className="grid grid-cols-12 gap-6 border-l border-border pl-8 md:gap-8"
          >
            <div className="col-span-12 flex items-baseline gap-4 md:col-span-3">
              <span className="font-mono text-sm text-text-dim">{step.num}</span>
              <span className="text-2xl text-accent">{step.glyph}</span>
              <span className="font-mono text-xs uppercase tracking-wider text-text-muted">
                Step
              </span>
            </div>
            <div className="col-span-12 md:col-span-9">
              <h3 className="text-2xl font-medium">{step.title}</h3>
              <p className="mt-2 text-text-muted">{step.body}</p>
              {step.sub.length > 0 && (
                <ul className="mt-4 space-y-2">
                  {step.sub.map((s, j) => (
                    <li
                      key={j}
                      className="flex items-start gap-3 text-sm text-text-muted"
                    >
                      <span className="mt-2 inline-block h-px w-4 bg-text-dim" />
                      <span>{s}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </motion.div>
        ))}
      </div>
    </SectionShell>
  );
}
