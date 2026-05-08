"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const THRESHOLDS = [
  {
    name: "max_deviation_bps",
    value: "2000",
    desc: "Max price change between updates (20%)",
  },
  {
    name: "max_staleness_seconds",
    value: "300",
    desc: "Max oracle data age (5 min)",
  },
  {
    name: "previous_max_staleness_seconds",
    value: "900",
    desc: "Max prev price age (15 min)",
  },
  {
    name: "min_liquidity_usd",
    value: "$10k",
    desc: "Min 30m SDEX volume threshold",
  },
  {
    name: "min_trade_count_1h",
    value: "5",
    desc: "Min unique trader count",
  },
  {
    name: "circuit_breaker_halt_ledgers",
    value: "720",
    desc: "Halt window on violation (~1h)",
  },
];

export function Mechanism() {
  return (
    <SectionShell id="mechanism" eyebrow="Mechanism">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7 }}
        className="text-5xl font-medium leading-[1.1] tracking-tight md:text-6xl"
      >
        Mathematically validated.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-8 max-w-2xl text-text-muted"
      >
        Every threshold below is calibrated for production deployment. Each is
        configurable per-integrator. Defaults reflect mainnet-ready security
        margins observed against real attack patterns.
      </motion.p>

      <div className="mt-16 divide-y divide-border border-y border-border">
        {THRESHOLDS.map((t, i) => (
          <motion.div
            key={t.name}
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: i * 0.05, duration: 0.4 }}
            className="grid grid-cols-12 gap-4 py-5"
          >
            <div className="col-span-12 font-mono text-sm text-text md:col-span-5">
              {t.name}
            </div>
            <div className="col-span-4 font-mono text-lg text-accent md:col-span-2">
              {t.value}
            </div>
            <div className="col-span-8 text-sm text-text-muted md:col-span-5">
              {t.desc}
            </div>
          </motion.div>
        ))}
      </div>
    </SectionShell>
  );
}
