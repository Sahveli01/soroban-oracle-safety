"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ATTACKS, MAX_DEVIATION_BPS, type Attack } from "@/lib/demo-data";
import { useReducedMotion } from "@/lib/use-reduced-motion";

const EASE = [0.19, 1, 0.22, 1] as const;
const THRESHOLD_PCT = MAX_DEVIATION_BPS / 100; // safe-oracle default = 20%
const LAST_BEAT = 5;
// Transition delays between beats: 0→1→2→3→4→5. Slow, weighted, cinematic.
const BEAT_MS = [750, 1050, 1150, 1150, 1250];

/** Real deviation of the manipulated peak vs. the last good price, in %. */
function devOf(a: Attack): number {
  return (a.manipPrice / a.prePrice - 1) * 100;
}
function money(n: number): string {
  return `$${n.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: n < 1 ? 4 : 2,
  })}`;
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
  const [beat, setBeat] = useState(0);
  const [playing, setPlaying] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const attack = ATTACKS[idx];
  const dev = devOf(attack);
  const spiked = beat >= 2; // oracle has reported the manipulated price
  const resolved = beat >= LAST_BEAT;

  const clear = useCallback(() => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
  }, []);

  const play = useCallback(() => {
    clear();
    if (reduced) {
      setBeat(LAST_BEAT);
      setPlaying(false);
      return;
    }
    setPlaying(true);
    setBeat(0);
    let b = 0;
    const step = () => {
      if (b >= LAST_BEAT) {
        setPlaying(false);
        return;
      }
      timer.current = setTimeout(() => {
        b += 1;
        setBeat(b);
        step();
      }, BEAT_MS[b]);
    };
    step();
  }, [clear, reduced]);

  // Switch scene → reset and auto-play the new heist.
  function selectAttack(i: number) {
    if (i === idx) return;
    clear();
    setIdx(i);
    setBeat(0);
    setPlaying(false);
    // let the new scene mount, then roll
    timer.current = setTimeout(play, 220);
  }

  useEffect(() => clear, [clear]);

  // ── Per-beat narration, one short line per side ───────────────────────
  const leftLine = [
    "monitoring price feed",
    attack.trigger,
    `feed jumps to ${money(attack.manipPrice)}`,
    "protocol accepts the price",
    "borrowing against phantom collateral",
    "position drained",
  ][beat];
  const rightLine = [
    "monitoring price feed",
    attack.trigger,
    `feed jumps to ${money(attack.manipPrice)}`,
    `deviation ${pct(dev)} exceeds ${THRESHOLD_PCT.toFixed(0)}%`,
    "bad price never served",
    "boundary held",
  ][beat];

  return (
    <div className="overflow-hidden rounded-2xl border border-border bg-[#08080d]">
      {/* grain-less inner cinematic frame */}
      <div className="relative p-5 sm:p-7">
        {/* ── Scene selector ─────────────────────────────────── */}
        <div className="flex flex-wrap items-center gap-2">
          {ATTACKS.map((a, i) => {
            const active = i === idx;
            return (
              <button
                key={a.id}
                onClick={() => selectAttack(i)}
                className={`tactile rounded-full border px-4 py-1.5 font-mono text-[12px] ${
                  active
                    ? "border-accent bg-accent-muted text-text"
                    : "border-border text-text-muted hover:border-border-strong"
                }`}
              >
                {a.name.split(" / ")[0]}
              </button>
            );
          })}
          <span className="ml-auto hidden font-mono text-[10px] uppercase tracking-[0.2em] text-text-dim sm:inline">
            Forensic replay
          </span>
        </div>

        {/* ── Scene header ───────────────────────────────────── */}
        <div className="mt-6">
          <div className="font-mono text-[11px] uppercase tracking-[0.22em] text-text-dim">
            {attack.date} · {attack.pair} · {attack.chain.replace(" (adapted)", "")}
          </div>
          <h3 className="mt-2 text-[28px] font-medium leading-none tracking-tight text-text sm:text-[34px]">
            {attack.name.split(" / ")[0]}
            <span className="text-text-dim"> — the </span>
            <span className="tabular text-danger">{usd(attack.lossUsd)}</span>
            <span className="text-text-dim"> heist</span>
          </h3>
          <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-text-muted">
            {attack.multiple} oracle spike on a {money(attack.prePrice)} asset.
            Same feed, two protocols — watch where their fates split.
          </p>
        </div>

        {/* ── Split screen ───────────────────────────────────── */}
        <div className="mt-6 grid grid-cols-1 gap-4 md:grid-cols-2">
          {/* LEFT — unprotected */}
          <motion.div
            className="relative overflow-hidden rounded-xl border p-5"
            animate={{
              borderColor: spiked ? "rgba(255,45,85,0.5)" : "rgba(26,26,36,1)",
              backgroundColor: spiked ? "rgba(255,45,85,0.05)" : "rgba(16,16,23,0.6)",
            }}
            transition={{ duration: 0.6, ease: EASE }}
          >
            {/* danger bloom */}
            <motion.div
              className="pointer-events-none absolute inset-0"
              style={{
                background:
                  "radial-gradient(120% 80% at 50% 120%, rgba(255,45,85,0.25), transparent 70%)",
              }}
              animate={{ opacity: resolved ? 0.9 : spiked ? 0.5 : 0 }}
              transition={{ duration: 0.7, ease: EASE }}
            />
            <PanelHead
              kicker="Unprotected protocol"
              sub="trusts the feed"
              tone="danger"
            />
            <PriceBlock
              attack={attack}
              spiked={spiked}
              reduced={reduced}
              struck={false}
            />
            <BeatLine line={leftLine} tone={spiked ? "danger" : "muted"} />
            <Stamp
              show={resolved}
              label="DRAINED"
              value={`− ${usd(attack.lossUsd)}`}
              tone="danger"
              glyph="✕"
              reduced={reduced}
            />
          </motion.div>

          {/* RIGHT — safe-oracle */}
          <motion.div
            className="relative overflow-hidden rounded-xl border p-5"
            animate={{
              borderColor:
                beat >= 3 ? "rgba(0,255,148,0.5)" : "rgba(26,26,36,1)",
              backgroundColor:
                beat >= 3 ? "rgba(0,255,148,0.05)" : "rgba(16,16,23,0.6)",
            }}
            transition={{ duration: 0.6, ease: EASE }}
          >
            <motion.div
              className="pointer-events-none absolute inset-0"
              style={{
                background:
                  "radial-gradient(120% 80% at 50% 120%, rgba(0,255,148,0.22), transparent 70%)",
              }}
              animate={{ opacity: resolved ? 0.9 : beat >= 3 ? 0.45 : 0 }}
              transition={{ duration: 0.7, ease: EASE }}
            />
            <PanelHead
              kicker="With safe-oracle"
              sub="validates the feed"
              tone="accent"
            />
            <PriceBlock
              attack={attack}
              spiked={spiked}
              reduced={reduced}
              struck={beat >= 3}
            />
            {/* Guardrail interception overlay */}
            <AnimatePresence>
              {beat >= 3 && !resolved && (
                <motion.div
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0 }}
                  className="mt-3 inline-flex items-center gap-2 rounded-md border border-accent/40 bg-accent-muted px-3 py-1.5 font-mono text-[12px] text-accent"
                >
                  ✓ guardrail engaged · {pct(dev)} &gt; {THRESHOLD_PCT.toFixed(0)}%
                </motion.div>
              )}
            </AnimatePresence>
            <BeatLine line={rightLine} tone={beat >= 3 ? "accent" : "muted"} />
            <Stamp
              show={resolved}
              label="PROTECTED"
              value={`${usd(attack.lossUsd)} safe`}
              tone="accent"
              glyph="🛡"
              reduced={reduced}
            />
          </motion.div>
        </div>

        {/* ── Beat rail + transport ──────────────────────────── */}
        <div className="mt-6 flex items-center gap-4">
          <button
            onClick={play}
            className="btn-secondary shrink-0 text-center text-[13px]"
            style={{ cursor: "pointer" }}
          >
            {playing ? "● Replaying…" : resolved ? "↻ Replay the heist" : "▶ Replay the heist"}
          </button>
          <div className="flex flex-1 items-center gap-1.5">
            {Array.from({ length: LAST_BEAT + 1 }).map((_, i) => (
              <div
                key={i}
                className="h-1 flex-1 overflow-hidden rounded-full bg-border"
              >
                <motion.div
                  className="h-full rounded-full"
                  initial={false}
                  animate={{
                    width: i <= beat ? "100%" : "0%",
                    backgroundColor:
                      i <= beat
                        ? i >= 3
                          ? "#00ff94"
                          : "#ff2d55"
                        : "transparent",
                  }}
                  transition={{ duration: 0.4, ease: EASE }}
                />
              </div>
            ))}
          </div>
        </div>

        {/* ── Honest label ───────────────────────────────────── */}
        <p className="mt-4 font-mono text-[10px] leading-relaxed text-text-dim">
          Real attack data · public post-mortems ({attack.source}) · replayed
          against safe-oracle&apos;s default {THRESHOLD_PCT.toFixed(0)}% deviation
          guardrail. Prices are the two real reference points (pre &amp; peak), not
          a simulated series.
        </p>
      </div>
    </div>
  );
}

/* ── Sub-components ────────────────────────────────────────────────────── */

function PanelHead({
  kicker,
  sub,
  tone,
}: {
  kicker: string;
  sub: string;
  tone: "danger" | "accent";
}) {
  const c = tone === "danger" ? "#ff2d55" : "#00ff94";
  return (
    <div className="relative flex items-center justify-between">
      <div className="flex items-center gap-2">
        <span
          className="inline-block h-1.5 w-1.5 rounded-full"
          style={{ background: c, boxShadow: `0 0 8px ${c}` }}
        />
        <span className="font-mono text-[11px] uppercase tracking-[0.18em] text-text">
          {kicker}
        </span>
      </div>
      <span className="font-mono text-[10px] text-text-dim">{sub}</span>
    </div>
  );
}

function PriceBlock({
  attack,
  spiked,
  struck,
  reduced,
}: {
  attack: Attack;
  spiked: boolean;
  struck: boolean;
  reduced: boolean;
}) {
  const shown = spiked ? attack.manipPrice : attack.prePrice;
  return (
    <div className="relative mt-4">
      <div className="text-[10px] uppercase tracking-wider text-text-dim">
        oracle price
      </div>
      <div className="relative mt-1 h-10">
        <AnimatePresence mode="wait">
          <motion.div
            key={spiked ? "manip" : "pre"}
            initial={
              spiked && !reduced
                ? { scale: 1.35, x: -6, opacity: 0 }
                : { opacity: 0 }
            }
            animate={{ scale: 1, x: 0, opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.45, ease: EASE }}
            className={`tabular font-mono text-[30px] font-semibold leading-none ${
              struck ? "line-through decoration-2" : ""
            }`}
            style={{
              color: spiked ? "#ff2d55" : "#f5f5f7",
              textDecorationColor: "#00ff94",
            }}
          >
            {money(shown)}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

function BeatLine({
  line,
  tone,
}: {
  line: string;
  tone: "danger" | "accent" | "muted";
}) {
  const c =
    tone === "danger" ? "#ff8895" : tone === "accent" ? "#7af0c0" : "#8e8e93";
  return (
    <div className="mt-4 h-5">
      <AnimatePresence mode="wait">
        <motion.div
          key={line}
          initial={{ opacity: 0, y: 5 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -5 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="font-mono text-[12px]"
          style={{ color: c }}
        >
          {line}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}

function Stamp({
  show,
  label,
  value,
  tone,
  glyph,
  reduced,
}: {
  show: boolean;
  label: string;
  value: string;
  tone: "danger" | "accent";
  glyph: string;
  reduced: boolean;
}) {
  const c = tone === "danger" ? "#ff2d55" : "#00ff94";
  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={reduced ? { opacity: 0 } : { opacity: 0, scale: 1.3, y: 4 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
          className="mt-5 flex items-center gap-3"
        >
          <span
            className="grid h-9 w-9 place-items-center rounded-lg text-[16px]"
            style={{ background: `${c}1a`, border: `1px solid ${c}55` }}
          >
            {glyph}
          </span>
          <div>
            <div
              className="font-mono text-[15px] font-semibold tracking-wide"
              style={{ color: c }}
            >
              {label}
            </div>
            <div className="tabular font-mono text-[12px] text-text-muted">
              {value}
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
