"use client";

import {
  motion,
  useInView,
  useMotionValue,
  useSpring,
  useTransform,
} from "framer-motion";
import { useRef, useEffect } from "react";
import { useReducedMotion } from "@/lib/use-reduced-motion";

/**
 * Hero stats row — four numbers that count up when scrolled into view.
 *
 * Numbers are pulled from public sources / project state:
 *   - $10.2M: YieldBlox post-mortem (Feb 22, 2026)
 *   - 316: workspace test count (current)
 *   - 0: critical findings (AR.H adversarial review)
 *   - 3: trustless Layer 1 guards active by default (lib.rs:379,
 *        layer2_enabled: false). Two more activate with opt-in Layer 2.
 */
const STATS = [
  {
    value: 10.2,
    prefix: "$",
    suffix: "M",
    label: "Drained",
    subtitle: "YieldBlox",
    decimals: 1,
  },
  { value: 316, label: "Tests", subtitle: "Passing" },
  { value: 0, label: "Critical", subtitle: "Findings" },
  { value: 3, label: "Guards", subtitle: "Trustless" },
];

function Stat({
  value,
  label,
  subtitle,
  prefix = "",
  suffix = "",
  decimals = 0,
}: {
  value: number;
  label: string;
  subtitle: string;
  prefix?: string;
  suffix?: string;
  decimals?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { once: true, margin: "-50px" });
  const reducedMotion = useReducedMotion();

  const motionValue = useMotionValue(0);
  const spring = useSpring(
    motionValue,
    reducedMotion ? { duration: 0, bounce: 0 } : { duration: 1500, bounce: 0 },
  );
  const display = useTransform(spring, (current) =>
    decimals > 0 ? current.toFixed(decimals) : Math.floor(current).toString(),
  );

  useEffect(() => {
    if (inView) motionValue.set(value);
  }, [inView, motionValue, value]);

  return (
    <div ref={ref} className="flex flex-col items-center gap-0.5">
      <div className="font-mono text-3xl font-medium tabular md:text-4xl">
        <span className="text-text-dim">{prefix}</span>
        <motion.span className="text-text">{display}</motion.span>
        <span className="text-text-dim">{suffix}</span>
      </div>
      <div className="font-mono text-[11px] uppercase tracking-wider text-text-muted">
        {label}
      </div>
      <div className="font-mono text-[11px] text-text-dim">{subtitle}</div>
    </div>
  );
}

export function StatsRow() {
  return (
    <div className="grid grid-cols-2 gap-6 md:grid-cols-4">
      {STATS.map((stat, i) => (
        <Stat key={i} {...stat} />
      ))}
    </div>
  );
}
