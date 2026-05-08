"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const DIAGRAM = `        Integrator              Library                  External
        ──────────              ────────                  ────────

    your_contract
        │
        │ lastprice()
        ▼
    ┌─────────────────┐
    │   safe_oracle   │
    │                 │
    │   ┌─────────┐   │      decimals(), lastprice()
    │   │ Layer 1 │   │ ───────────────────────────► Reflector
    │   └─────────┘   │
    │                 │      get_snapshot(asset)
    │   ┌─────────┐   │ ───────────────────────────► LiquidityRegistry
    │   │ Layer 2 │   │
    │   └─────────┘   │
    │                 │
    │   ┌─────────┐   │
    │   │   CB    │   │
    │   └─────────┘   │
    └────────┬────────┘
             │
             ▼
        Ok(price) │ Err(violation)
             │
             ▼
        use price`;

export function Architecture() {
  return (
    <SectionShell id="architecture" eyebrow="Architecture">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7 }}
        className="text-4xl font-medium leading-[1.1] tracking-tight sm:text-5xl md:text-6xl"
      >
        Purely defensive.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-8 max-w-2xl text-text-muted"
      >
        safe-oracle doesn&apos;t replace Reflector or Stellar&apos;s built-in
        price feeds. It validates them. The library sits between your contract
        and the oracle, running every guardrail before the price reaches your
        business logic.
      </motion.p>

      <motion.div
        initial={{ opacity: 0, y: 30 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-50px" }}
        transition={{ delay: 0.3, duration: 0.8 }}
        className="mt-16 overflow-x-auto rounded-xl border border-border bg-surface p-6 md:p-8"
      >
        <pre className="font-mono text-xs leading-relaxed text-text-muted md:text-sm">
          {DIAGRAM}
        </pre>
      </motion.div>

      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.5, duration: 0.6 }}
        className="mt-12 grid gap-6 text-sm md:grid-cols-2"
      >
        <div className="rounded-lg border border-border bg-surface p-6">
          <div className="font-mono text-xs uppercase tracking-wider text-text-muted">
            On-Chain
          </div>
          <div className="mt-2 text-text">
            safe-oracle library + LiquidityRegistry contract
          </div>
          <div className="mt-1 text-text-muted">
            Embedded in your contract. Validates every oracle call.
          </div>
        </div>
        <div className="rounded-lg border border-border bg-surface p-6">
          <div className="font-mono text-xs uppercase tracking-wider text-text-muted">
            Off-Chain
          </div>
          <div className="mt-2 text-text">oracle-watch service</div>
          <div className="mt-1 text-text-muted">
            Monitors SDEX, signs liquidity snapshots, alerts on anomalies.
          </div>
        </div>
      </motion.div>
    </SectionShell>
  );
}
