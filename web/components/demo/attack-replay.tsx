"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  ReferenceLine,
  Tooltip,
} from "recharts";
import { ATTACKS, MAX_DEVIATION_BPS, type Attack } from "@/lib/demo-data";

const EASE = [0.19, 1, 0.22, 1] as const;
const BASELINE = 7; // flat pre-attack ticks before the manipulated spike

function buildSeries(a: Attack) {
  const pts: { i: number; price: number; spike?: boolean }[] = [];
  for (let i = 0; i < BASELINE; i++) pts.push({ i, price: a.prePrice });
  pts.push({ i: BASELINE, price: a.manipPrice, spike: true });
  return pts;
}

function usd(n: number) {
  if (n >= 1_000_000)
    return `$${(n / 1_000_000).toLocaleString("en-US", { maximumFractionDigits: 1 })}M`;
  return `$${n.toLocaleString("en-US")}`;
}

export default function AttackReplay() {
  const [idx, setIdx] = useState(0);
  const attack = ATTACKS[idx];
  const series = useMemo(() => buildSeries(attack), [attack]);
  const threshold = attack.prePrice * (1 + MAX_DEVIATION_BPS / 10_000);

  const [visible, setVisible] = useState(1);
  const [playing, setPlaying] = useState(false);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  const done = visible >= series.length;
  const spikeShown = done;

  function reset() {
    if (timer.current) clearInterval(timer.current);
    setPlaying(false);
    setVisible(1);
  }

  function play() {
    reset();
    setPlaying(true);
    setVisible(1);
    timer.current = setInterval(() => {
      setVisible((v) => {
        if (v >= series.length) {
          if (timer.current) clearInterval(timer.current);
          setPlaying(false);
          return v;
        }
        return v + 1;
      });
    }, 340);
  }

  // reset when switching attack
  useEffect(() => reset, [idx]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => () => void (timer.current && clearInterval(timer.current)), []);

  const data = series.slice(0, visible);

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[0.85fr_1.15fr]">
      {/* ── Attack picker + verdict ──────────────────────────── */}
      <div className="surface-card p-6 sm:p-7">
        <p className="t-eyebrow mb-5">Replay a real attack</p>

        <div className="space-y-2">
          {ATTACKS.map((a, i) => {
            const active = i === idx;
            return (
              <button
                key={a.id}
                onClick={() => setIdx(i)}
                className={`block w-full rounded-lg border px-4 py-3 text-left transition-all duration-300 ${
                  active
                    ? "border-accent bg-accent-muted"
                    : "border-border hover:border-border-strong"
                }`}
              >
                <div className="flex items-baseline justify-between">
                  <span className="text-sm text-text">{a.name}</span>
                  <span className="tabular font-mono text-[13px] text-danger">
                    {usd(a.lossUsd)}
                  </span>
                </div>
                <div className="mt-0.5 font-mono text-[11px] text-text-dim">
                  {a.pair} · {a.date} · {a.chain}
                </div>
              </button>
            );
          })}
        </div>

        <button
          onClick={playing ? reset : play}
          className="btn-secondary mt-6 w-full text-center"
          style={{ cursor: "pointer" }}
        >
          {playing ? "Stop" : done ? "Replay ↻" : "▶ Play attack"}
        </button>

        <p className="mt-4 text-[11px] leading-relaxed text-text-dim">
          Real post-mortem prices replayed against safe-oracle&apos;s default{" "}
          {(MAX_DEVIATION_BPS / 100).toFixed(0)}% deviation threshold.{" "}
          {attack.sourceNote}
        </p>
      </div>

      {/* ── Chart + outcome ──────────────────────────────────── */}
      <div className="surface-card p-6 sm:p-7">
        <div className="mb-2 flex items-baseline justify-between">
          <p className="t-eyebrow">{attack.pair} oracle price</p>
          <span className="font-mono text-[11px] text-text-dim">log scale</span>
        </div>

        <div className="h-56 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 12, right: 12, bottom: 4, left: 4 }}>
              <XAxis dataKey="i" hide />
              <YAxis
                scale="log"
                domain={[
                  Math.min(attack.prePrice, threshold) * 0.6,
                  attack.manipPrice * 1.4,
                ]}
                tick={{ fill: "#7a7a82", fontSize: 11, fontFamily: "var(--font-mono)" }}
                tickFormatter={(v: number) =>
                  v >= 1000 ? `$${(v / 1000).toFixed(0)}k` : `$${v}`
                }
                width={48}
                axisLine={false}
                tickLine={false}
              />
              <Tooltip
                contentStyle={{
                  background: "#101017",
                  border: "1px solid #272733",
                  borderRadius: 8,
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                }}
                labelFormatter={() => ""}
                formatter={(v) => [`$${Number(v).toLocaleString()}`, "price"]}
              />
              <ReferenceLine
                y={threshold}
                stroke="#ff2d55"
                strokeDasharray="4 4"
                strokeOpacity={0.7}
                label={{
                  value: "guardrail threshold",
                  fill: "#ff2d55",
                  fontSize: 10,
                  fontFamily: "var(--font-mono)",
                  position: "insideTopLeft",
                }}
              />
              <Line
                type="monotone"
                dataKey="price"
                stroke="#00ff94"
                strokeWidth={2}
                isAnimationActive={false}
                dot={(props: { cx?: number; cy?: number; payload?: { spike?: boolean } }) => {
                  const { cx, cy, payload } = props;
                  if (cx == null || cy == null) return <g />;
                  const spike = payload?.spike;
                  return (
                    <circle
                      cx={cx}
                      cy={cy}
                      r={spike ? 5 : 2.5}
                      fill={spike ? "#ff2d55" : "#00ff94"}
                    />
                  );
                }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Outcome comparison */}
        <div className="mt-5 grid grid-cols-2 gap-3">
          <div className="rounded-lg border border-border bg-surface px-4 py-3">
            <div className="text-[11px] uppercase tracking-wider text-text-dim">
              Unprotected
            </div>
            <AnimatePresence mode="wait">
              <motion.div
                key={spikeShown ? "drained" : "ok1"}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="mt-1 font-mono text-sm"
                style={{ color: spikeShown ? "#ff2d55" : "#8e8e93" }}
              >
                {spikeShown ? `drained ${usd(attack.lossUsd)}` : "feed accepted"}
              </motion.div>
            </AnimatePresence>
          </div>
          <div className="rounded-lg border border-border bg-surface px-4 py-3">
            <div className="text-[11px] uppercase tracking-wider text-text-dim">
              With safe-oracle
            </div>
            <AnimatePresence mode="wait">
              <motion.div
                key={spikeShown ? "blocked" : "ok2"}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="mt-1 font-mono text-sm"
                style={{ color: "#00ff94" }}
              >
                {spikeShown ? "✓ REJECTED — Excessive Deviation" : "monitoring"}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>

        <AnimatePresence>
          {spikeShown && (
            <motion.p
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ ease: EASE }}
              className="mt-4 text-[12px] leading-relaxed text-text-muted"
            >
              The {attack.multiple} jump blows past the threshold —{" "}
              <span className="text-accent">{usd(attack.lossUsd)}</span> would
              have been blocked at the oracle boundary. {attack.summary}
            </motion.p>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
