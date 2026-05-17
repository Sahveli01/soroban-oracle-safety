"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const STEPS = [
  {
    num: "01",
    title: "Reflector Call",
    body: "Your contract calls safe_oracle::lastprice() instead of Reflector directly.",
  },
  {
    num: "02",
    title: "Layer 1 — Oracle Checks",
    body: "Deviation, staleness and cross-source disagreement validated against feed mechanics.",
  },
  {
    num: "03",
    title: "Layer 2 — Market Checks",
    body: "SDEX 30-minute volume and unique-trader count validated against on-chain liquidity.",
  },
  {
    num: "04",
    title: "Circuit Breaker",
    body: "Auto-halt after the first violation. Governance manual override available.",
  },
  {
    num: "05",
    title: "Result",
    body: "Validated price returned — or Err with the specific violation type.",
  },
];

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

export function HowItWorks() {
  return (
    <SectionShell id="how-it-works" eyebrow="How It Works">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: EASE }}
        className="t-h2"
      >
        Five steps.{" "}
        <span className="text-accent">One result.</span>
      </motion.h2>

      {/* Compact connected timeline — balanced to the viewport */}
      <div className="mt-14 grid gap-x-6 gap-y-px sm:grid-cols-2 lg:grid-cols-5">
        {STEPS.map((step, i) => (
          <motion.div
            key={step.num}
            initial={{ opacity: 0, y: 18 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: i * 0.08, duration: 0.55, ease: EASE }}
            className="group relative flex flex-col rounded-xl border border-border bg-surface p-5 transition-colors hover:border-accent/40"
          >
            <div className="flex items-center justify-between">
              <span className="font-mono text-2xl font-medium text-accent tabular">
                {step.num}
              </span>
              <span className="h-px w-8 bg-border transition-colors group-hover:bg-accent/50" />
            </div>
            <h3 className="mt-6 text-lg font-medium leading-snug">
              {step.title}
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-text-muted">
              {step.body}
            </p>
          </motion.div>
        ))}
      </div>
    </SectionShell>
  );
}
