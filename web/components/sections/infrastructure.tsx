"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const CARDS = [
  {
    tag: "On-chain",
    title: "Layer 1 Guardrails",
    desc: "Deviation, staleness, cross-source. Validates oracle output before it reaches your logic.",
  },
  {
    tag: "On-chain",
    title: "Layer 2 Guardrails",
    desc: "Liquidity volume + thin sampling. Validates market microstructure on-chain.",
  },
  {
    tag: "On-chain",
    title: "Circuit Breaker",
    desc: "Auto-halt on first violation. Per-asset isolation. Manual governance override.",
  },
  {
    tag: "On-chain",
    title: "Liquidity Registry",
    desc: "Signed snapshots from off-chain attesters. Authoritative source for Layer 2 checks.",
  },
  {
    tag: "Off-chain",
    title: "oracle-watch",
    desc: "Rust service. Monitors SDEX, signs snapshots, dispatches Slack / PagerDuty / webhook alerts.",
  },
  {
    tag: "Platform",
    title: "Soroban-Native",
    desc: "Built for Stellar Soroban 25. WASM contract + reqwest off-chain. No bridges.",
  },
];

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

export function Infrastructure() {
  return (
    <SectionShell id="infrastructure" eyebrow="Infrastructure">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: EASE }}
        className="text-4xl font-medium leading-[1.05] tracking-tight sm:text-5xl md:text-6xl"
      >
        Modular by design.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.15, duration: 0.7 }}
        className="mt-5 max-w-xl text-text-muted"
      >
        Six independent components. Adopt the whole stack or only the guard you
        need — each is a clean, isolated boundary.
      </motion.p>

      <div className="mt-12 grid gap-3 md:grid-cols-2 lg:grid-cols-3">
        {CARDS.map((card, i) => (
          <motion.div
            key={card.title}
            initial={{ opacity: 0, y: 18 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: i * 0.06, duration: 0.5, ease: EASE }}
            className="surface-card group flex flex-col p-6"
          >
            <div className="flex items-center justify-between">
              <span className="font-mono text-[11px] tabular text-text-dim">
                {String(i + 1).padStart(2, "0")}
              </span>
              <span className="rounded-full border border-border px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-text-dim transition-colors group-hover:border-accent/40 group-hover:text-accent">
                {card.tag}
              </span>
            </div>
            <h3 className="mt-8 text-xl font-medium tracking-tight">
              {card.title}
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-text-muted">
              {card.desc}
            </p>
          </motion.div>
        ))}
      </div>
    </SectionShell>
  );
}
