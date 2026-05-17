"use client";

import {
  motion,
  useInView,
  useMotionValue,
  useSpring,
  useTransform,
} from "framer-motion";
import { useRef, useEffect } from "react";
import { SectionShell } from "./section-shell";

type FindingColor = "accent" | undefined;

const FINDINGS: {
  count: number;
  label: string;
  suffix?: string;
  color?: FindingColor;
}[] = [
  { count: 20, label: "Attack vectors attempted", suffix: "+" },
  { count: 0, label: "Critical findings", color: "accent" },
  { count: 0, label: "High findings", color: "accent" },
  { count: 3, label: "Medium (closed in 50 min)" },
  { count: 5, label: "Low" },
  { count: 10, label: "Info" },
];

function FindingRow({
  count,
  label,
  suffix = "",
  color,
}: {
  count: number;
  label: string;
  suffix?: string;
  color?: FindingColor;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { once: true, margin: "-50px" });

  const motionValue = useMotionValue(0);
  const spring = useSpring(motionValue, { duration: 1500, bounce: 0 });
  const display = useTransform(spring, (current) =>
    Math.floor(current).toString(),
  );

  useEffect(() => {
    if (inView) motionValue.set(count);
  }, [inView, motionValue, count]);

  return (
    <div
      ref={ref}
      className="grid grid-cols-12 gap-4 border-b border-border py-5"
    >
      <div
        className={`col-span-3 font-mono text-3xl font-medium tabular md:col-span-2 md:text-4xl ${
          color === "accent" ? "text-accent" : "text-text"
        }`}
      >
        <motion.span>{display}</motion.span>
        <span className="text-text-dim">{suffix}</span>
      </div>
      <div className="col-span-9 self-center text-text-muted md:col-span-10">
        {label}
      </div>
    </div>
  );
}

export function Audit() {
  return (
    <SectionShell id="audit" eyebrow="Adversarial Review">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7 }}
        className="t-h2"
      >
        Independently audited.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-8 max-w-2xl text-text-muted"
      >
        safe-oracle underwent independent adversarial review (AR.H) attempting
        20+ distinct attack vectors across all five guardrails. Median patch
        time: 50 minutes. Zero criticals. Zero highs.
      </motion.p>

      <div className="mt-16 border-t border-border">
        {FINDINGS.map((f, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true, margin: "-30px" }}
            transition={{ delay: i * 0.08, duration: 0.4 }}
          >
            <FindingRow {...f} />
          </motion.div>
        ))}
      </div>
    </SectionShell>
  );
}
