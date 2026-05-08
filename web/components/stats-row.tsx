"use client";

import {
  motion,
  useInView,
  useMotionValue,
  useSpring,
  useTransform,
} from "framer-motion";
import { useRef, useEffect } from "react";

/**
 * Hero stats row — four numbers that count up when scrolled into view.
 *
 * Numbers are pulled from public sources / project state:
 *   - $10.2M: YieldBlox post-mortem (Feb 22, 2026)
 *   - 290: workspace test count at Phase 7 closure
 *   - 0: critical findings (AR.H adversarial review)
 *   - 5: active guardrails (Layers 1+2)
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
  { value: 290, label: "Tests", subtitle: "Passing" },
  { value: 0, label: "Critical", subtitle: "Findings" },
  { value: 5, label: "Guards", subtitle: "Active" },
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

  const motionValue = useMotionValue(0);
  const spring = useSpring(motionValue, { duration: 1500, bounce: 0 });
  const display = useTransform(spring, (current) =>
    decimals > 0 ? current.toFixed(decimals) : Math.floor(current).toString(),
  );

  useEffect(() => {
    if (inView) motionValue.set(value);
  }, [inView, motionValue, value]);

  return (
    <div ref={ref} className="flex flex-col items-center gap-1">
      <div className="font-mono text-4xl font-medium tabular md:text-5xl">
        <span className="text-text-dim">{prefix}</span>
        <motion.span className="text-text">{display}</motion.span>
        <span className="text-text-dim">{suffix}</span>
      </div>
      <div className="font-mono text-xs uppercase tracking-wider text-text-muted">
        {label}
      </div>
      <div className="font-mono text-xs text-text-dim">{subtitle}</div>
    </div>
  );
}

export function StatsRow() {
  return (
    <div className="grid grid-cols-2 gap-8 md:grid-cols-4">
      {STATS.map((stat, i) => (
        <Stat key={i} {...stat} />
      ))}
    </div>
  );
}
