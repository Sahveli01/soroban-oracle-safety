"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ATTACKS, MAX_DEVIATION_BPS, type Attack } from "@/lib/demo-data";
import { useReducedMotion } from "@/lib/use-reduced-motion";

const EASE = [0.19, 1, 0.22, 1] as const;
const THRESHOLD_PCT = MAX_DEVIATION_BPS / 100; // safe-oracle default = 20%
const BOUNDARY_T = 0.3; // the +20% line sits 30% along the track…
const DUR = 1500; // …so the rest of the track holds the huge real spike (log).

/** Real deviation of the manipulated peak vs. the last good price, in %. */
function realDev(a: Attack): number {
  return (a.manipPrice / a.prePrice - 1) * 100;
}

/** Track position (0..1) → deviation %. Linear up to the boundary, log past
 *  it so a 100×–100,000× spike still fits on one track. Axis tops out at 2×
 *  the real spike, so the real attack lands near (but not at) the far edge. */
function devFromPos(t: number, axisMax: number): number {
  if (t <= BOUNDARY_T) return THRESHOLD_PCT * (t / BOUNDARY_T);
  const k = (t - BOUNDARY_T) / (1 - BOUNDARY_T);
  return THRESHOLD_PCT * Math.pow(axisMax / THRESHOLD_PCT, k);
}
function posFromDev(dev: number, axisMax: number): number {
  if (dev <= THRESHOLD_PCT) return BOUNDARY_T * (dev / THRESHOLD_PCT);
  const k = Math.log(dev / THRESHOLD_PCT) / Math.log(axisMax / THRESHOLD_PCT);
  return BOUNDARY_T + (1 - BOUNDARY_T) * k;
}

function usd(n: number): string {
  if (n >= 1_000_000)
    return `$${(n / 1_000_000).toLocaleString("en-US", { maximumFractionDigits: 1 })}M`;
  return `$${n.toLocaleString("en-US")}`;
}
function pct(p: number): string {
  return `+${Math.round(p).toLocaleString("en-US")}%`;
}

export default function AttackReplay() {
  const reduced = useReducedMotion();
  const [idx, setIdx] = useState(0);
  const attack = ATTACKS[idx];

  const axisMax = realDev(attack) * 2;
  const realT = posFromDev(realDev(attack), axisMax);

  const [t, setT] = useState(0);
  const [playing, setPlaying] = useState(false);
  const raf = useRef<number | null>(null);
  const trackRef = useRef<HTMLDivElement | null>(null);

  const dev = devFromPos(t, axisMax);
  const crossed = dev >= THRESHOLD_PCT;
  const color = crossed ? "#ff2d55" : "#00ff94";

  const cancel = useCallback(() => {
    if (raf.current != null) cancelAnimationFrame(raf.current);
    raf.current = null;
  }, []);

  const play = useCallback(() => {
    cancel();
    if (reduced) {
      setT(realT);
      setPlaying(false);
      return;
    }
    setPlaying(true);
    setT(0);
    const start = performance.now();
    const tick = (now: number) => {
      const e = Math.min(1, (now - start) / DUR);
      const eased = 1 - Math.pow(1 - e, 4); // easeOutQuart
      setT(realT * eased);
      if (e < 1) raf.current = requestAnimationFrame(tick);
      else {
        raf.current = null;
        setPlaying(false);
      }
    };
    raf.current = requestAnimationFrame(tick);
  }, [cancel, reduced, realT]);

  function reset() {
    cancel();
    setPlaying(false);
    setT(0);
  }

  // Reset whenever the selected attack changes; clean up on unmount.
  useEffect(() => {
    cancel();
    setPlaying(false);
    setT(0);
  }, [idx, cancel]);
  useEffect(() => cancel, [cancel]);

  // ── Drag: become the attacker, push the price yourself ──────────────
  const setFromClientX = useCallback((clientX: number) => {
    const el = trackRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const next = Math.min(1, Math.max(0, (clientX - r.left) / r.width));
    setT(next);
  }, []);

  function onPointerDown(e: React.PointerEvent) {
    cancel();
    setPlaying(false);
    (e.target as Element).setPointerCapture?.(e.pointerId);
    setFromClientX(e.clientX);
  }
  function onPointerMove(e: React.PointerEvent) {
    if (e.buttons === 0) return;
    setFromClientX(e.clientX);
  }

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[0.85fr_1.15fr]">
      {/* ── Attack picker ────────────────────────────────────── */}
      <div className="surface-card p-6 sm:p-7">
        <p className="t-eyebrow mb-5">Replay a real attack</p>

        <div className="space-y-2">
          {ATTACKS.map((a, i) => {
            const active = i === idx;
            return (
              <button
                key={a.id}
                onClick={() => setIdx(i)}
                className={`tactile block w-full rounded-lg border px-4 py-3 text-left ${
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
          {playing ? "Stop" : "▶ Run the real attack"}
        </button>

        <p className="mt-4 text-[11px] leading-relaxed text-text-dim">
          <span className="text-text-muted">Real attack, real prices.</span>{" "}
          Drag the dial to push the price yourself, or run the real spike —
          either way safe-oracle rejects past {THRESHOLD_PCT.toFixed(0)}%.{" "}
          {attack.sourceNote}
        </p>
      </div>

      {/* ── Boundary meter ───────────────────────────────────── */}
      <div className="surface-card p-6 sm:p-7">
        <div className="mb-1 flex items-baseline justify-between">
          <p className="t-eyebrow">{attack.pair} · deviation from last good price</p>
          <span className="font-mono text-[11px] text-text-dim">log axis</span>
        </div>

        {/* Live readout — the deviation the attacker is pushing right now */}
        <div className="mt-4 flex items-end justify-between">
          <motion.div
            key={crossed ? "rej" : "ok"}
            initial={{ scale: reduced ? 1 : 0.96, opacity: 0.6 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ duration: 0.25, ease: EASE }}
          >
            <div className="text-[11px] uppercase tracking-wider text-text-dim">
              {crossed ? "Rejected by safe-oracle" : "Within safe band"}
            </div>
            <div
              className="tabular mt-0.5 font-mono text-4xl font-semibold tracking-tight"
              style={{ color }}
            >
              {pct(dev)}
            </div>
          </motion.div>
          <div className="text-right">
            <div className="text-[11px] uppercase tracking-wider text-text-dim">
              Boundary
            </div>
            <div className="mt-0.5 font-mono text-lg text-text-muted">
              {THRESHOLD_PCT.toFixed(0)}%
            </div>
          </div>
        </div>

        {/* The dial */}
        <div className="mt-7 select-none">
          <div
            className="relative cursor-grab py-4 active:cursor-grabbing"
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            style={{ touchAction: "none" }}
          >
            {/* track */}
            <div
              ref={trackRef}
              className="relative h-2.5 w-full rounded-full"
              style={{
                background:
                  "linear-gradient(90deg, rgba(0,255,148,0.16) 0%, rgba(0,255,148,0.16) 28%, rgba(255,45,85,0.14) 34%, rgba(255,45,85,0.22) 100%)",
              }}
            >
              {/* fill */}
              <div
                className="absolute inset-y-0 left-0 rounded-full"
                style={{
                  width: `${t * 100}%`,
                  background: color,
                  boxShadow: `0 0 14px -2px ${color}`,
                  transition: playing ? "none" : "width 80ms linear",
                }}
              />
              {/* boundary line */}
              <motion.div
                className="absolute top-1/2 h-7 w-[2px] -translate-y-1/2"
                style={{ left: `${BOUNDARY_T * 100}%`, background: "#ff2d55" }}
                animate={
                  crossed && !reduced
                    ? { boxShadow: ["0 0 0px #ff2d55", "0 0 16px #ff2d55", "0 0 0px #ff2d55"] }
                    : {}
                }
                transition={{ duration: 1.4, repeat: crossed ? Infinity : 0 }}
              />
              {/* real-attack marker */}
              <div
                className="absolute top-1/2 -translate-x-1/2 -translate-y-1/2"
                style={{ left: `${realT * 100}%` }}
              >
                <span className="block text-[10px] leading-none text-text-dim">◆</span>
              </div>
              {/* handle */}
              <motion.div
                className="absolute top-1/2 h-5 w-5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2"
                style={{
                  left: `${t * 100}%`,
                  borderColor: color,
                  background: "#050507",
                  boxShadow: `0 0 12px -1px ${color}`,
                  transition: playing ? "none" : "left 80ms linear",
                }}
              />
            </div>

            {/* labels */}
            <div className="relative mt-3 h-8 font-mono text-[10px] text-text-dim">
              <span className="absolute left-0">0%</span>
              <span
                className="absolute -translate-x-1/2 text-center text-danger"
                style={{ left: `${BOUNDARY_T * 100}%` }}
              >
                safe-oracle
                <br />+{THRESHOLD_PCT.toFixed(0)}%
              </span>
              <span
                className="absolute -translate-x-1/2 text-center"
                style={{ left: `${realT * 100}%` }}
              >
                <span className="text-text-muted">{attack.multiple}</span>
                <br />real spike
              </span>
            </div>
          </div>
        </div>

        {/* Outcome chips */}
        <div className="mt-4 grid grid-cols-2 gap-3">
          <div className="rounded-lg border border-border bg-surface px-4 py-3">
            <div className="text-[11px] uppercase tracking-wider text-text-dim">
              Unprotected protocol
            </div>
            <AnimatePresence mode="wait">
              <motion.div
                key={crossed ? "drained" : "accept"}
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.2 }}
                className="mt-1 font-mono text-sm"
                style={{ color: crossed ? "#ff2d55" : "#8e8e93" }}
              >
                {crossed ? `drained ${usd(attack.lossUsd)}` : "feed accepted"}
              </motion.div>
            </AnimatePresence>
          </div>
          <motion.div
            className="rounded-lg border bg-surface px-4 py-3"
            animate={{
              borderColor: crossed ? "rgba(0,255,148,0.45)" : "rgba(26,26,36,1)",
            }}
            transition={{ duration: 0.3 }}
          >
            <div className="text-[11px] uppercase tracking-wider text-text-dim">
              With safe-oracle
            </div>
            <AnimatePresence mode="wait">
              <motion.div
                key={crossed ? "rejected" : "monitor"}
                initial={{ opacity: 0, scale: crossed && !reduced ? 0.9 : 1 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.22, ease: EASE }}
                className="mt-1 font-mono text-sm text-accent"
              >
                {crossed ? "✗ REJECTED — Excessive Deviation" : "✓ within band"}
              </motion.div>
            </AnimatePresence>
          </motion.div>
        </div>

        {/* Caption */}
        <AnimatePresence>
          {crossed && (
            <motion.p
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              transition={{ ease: EASE }}
              className="mt-4 text-[12px] leading-relaxed text-text-muted"
            >
              The {attack.multiple} jump blows past the {THRESHOLD_PCT.toFixed(0)}%
              boundary — <span className="text-accent">{usd(attack.lossUsd)}</span>{" "}
              stopped at the oracle layer. {attack.summary}
            </motion.p>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
