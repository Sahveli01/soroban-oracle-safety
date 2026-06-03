"use client";

import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ORACLES,
  ASSETS,
  GUARDRAILS,
  VIOLATIONS,
  formatPrice14,
  type OracleId,
  type AssetSym,
} from "@/lib/demo-data";

type Result = {
  approved?: boolean;
  price?: string;
  timestamp?: number;
  violation?: number;
  unavailable?: boolean;
  error?: string;
};

const EASE = [0.19, 1, 0.22, 1] as const;

// Real data-age thresholds (seconds) — mirror safe-oracle's staleness window.
const AGE_FRESH = 300; // < 5m  → fresh (green)
const AGE_STALE = 900; // < 15m → aging (amber); ≥ 15m → stale (red)

function ageColor(sec: number): string {
  if (sec < AGE_FRESH) return "#00ff94";
  if (sec < AGE_STALE) return "#f5b400";
  return "#ff2d55";
}
function ageLabel(sec: number): string {
  if (sec < AGE_FRESH) return "fresh";
  if (sec < AGE_STALE) return "aging";
  return "stale";
}
function formatAge(sec: number): string {
  if (sec < 60) return `${sec}s`;
  return `${Math.floor(sec / 60)}m ${sec % 60}s`;
}

/** Live data-age readout computed from the REAL on-chain price timestamp.
 *  Ticks every second; never fabricated — `timestamp` is the ledger value
 *  the validator returned. */
function DataAge({ timestamp }: { timestamp: number }) {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    const id = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(id);
  }, []);
  const age = Math.max(0, now - timestamp);
  const color = ageColor(age);
  return (
    <div className="text-right">
      <div className="text-[11px] uppercase tracking-wider text-text-dim">
        Data age
      </div>
      <div className="mt-1 flex items-center justify-end gap-1.5">
        <motion.span
          className="inline-block h-1.5 w-1.5 rounded-full"
          style={{ background: color }}
          animate={{ boxShadow: [`0 0 0px ${color}`, `0 0 8px ${color}`, `0 0 0px ${color}`] }}
          transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
        />
        <span className="tabular font-mono text-lg" style={{ color }}>
          {formatAge(age)}
        </span>
      </div>
      <div className="mt-0.5 font-mono text-[10px] text-text-dim">
        <span style={{ color }}>{ageLabel(age)}</span> · on-chain ts
      </div>
    </div>
  );
}

export default function MultiOracle() {
  const [oracle, setOracle] = useState<OracleId>("reflector");
  const [asset, setAsset] = useState<AssetSym>("BTC");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<Result | null>(null);

  const selected = ORACLES.find((o) => o.id === oracle)!;

  async function run() {
    setLoading(true);
    setResult(null);
    try {
      const res = await fetch("/api/validate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ oracle, asset }),
      });
      setResult(await res.json());
    } catch {
      setResult({ error: "network" });
    } finally {
      setLoading(false);
    }
  }

  const approved = result?.approved === true;
  const rejected = result?.approved === false;
  const failKey =
    rejected && result?.violation != null
      ? GUARDRAILS.find((g) =>
          (g.violations as readonly number[]).includes(result.violation!),
        )?.key
      : undefined;
  const failIdx = GUARDRAILS.findIndex((g) => g.key === failKey);

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[1fr_1.15fr]">
      {/* ── Controls ─────────────────────────────────────────── */}
      <div className="surface-card p-6 sm:p-7">
        <p className="t-eyebrow mb-5">Select a feed</p>

        <div className="grid grid-cols-2 gap-2.5">
          {ORACLES.map((o) => {
            const active = o.id === oracle;
            return (
              <button
                key={o.id}
                disabled={!o.live}
                onClick={() => {
                  setOracle(o.id);
                  setResult(null);
                }}
                className={`tactile relative rounded-lg border px-4 py-3.5 text-left ${
                  active
                    ? "border-accent bg-accent-muted"
                    : o.live
                      ? "border-border hover:border-border-strong"
                      : "border-border opacity-40"
                }`}
                style={{ cursor: o.live ? "pointer" : "not-allowed" }}
              >
                <span className="font-mono text-sm text-text">{o.label}</span>
                {!o.live && (
                  <span className="mt-1 block text-[11px] leading-tight text-text-dim">
                    not deployed at testnet address
                  </span>
                )}
                {o.live && (
                  <span
                    className={`mt-1 flex items-center gap-1.5 text-[11px] ${active ? "text-accent" : "text-text-dim"}`}
                  >
                    <span className="inline-block h-1.5 w-1.5 rounded-full bg-accent" />
                    testnet
                  </span>
                )}
              </button>
            );
          })}
        </div>

        <p className="t-eyebrow mb-3 mt-7">Asset</p>
        <div className="flex gap-2">
          {ASSETS.map((a) => (
            <button
              key={a}
              onClick={() => {
                setAsset(a);
                setResult(null);
              }}
              className={`tactile rounded-lg border px-5 py-2.5 font-mono text-sm ${
                a === asset
                  ? "border-accent bg-accent-muted text-text"
                  : "border-border text-text-muted hover:border-border-strong"
              }`}
            >
              {a}/USD
            </button>
          ))}
        </div>

        <button
          onClick={run}
          disabled={loading || !selected.live}
          className="btn-primary mt-7 w-full text-center disabled:opacity-50"
          style={{ cursor: loading ? "wait" : "pointer" }}
        >
          {loading ? "Validating on testnet…" : "Fetch & validate"}
        </button>

        <p className="mt-4 font-mono text-[11px] leading-relaxed text-text-dim">
          {selected.note}
        </p>
      </div>

      {/* ── Result ───────────────────────────────────────────── */}
      <div className="surface-card relative min-h-[24rem] overflow-hidden p-6 sm:p-7">
        <p className="t-eyebrow mb-5">Validator verdict</p>

        <AnimatePresence mode="wait">
          {!result && !loading && (
            <motion.div
              key="idle"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="flex h-64 flex-col items-center justify-center text-center"
            >
              <div className="font-mono text-sm text-text-dim">
                Read-only · simulateTransaction
              </div>
              <div className="mt-2 max-w-xs text-[13px] leading-relaxed text-text-muted">
                One safety layer, every oracle. Pick a feed and run the live
                Layer 1 guardrails.
              </div>
            </motion.div>
          )}

          {loading && (
            <motion.div
              key="loading"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              aria-busy="true"
            >
              {/* Skeleton mirrors the verdict layout so the result lands in
                  place instead of popping in. */}
              <div className="animate-pulse space-y-5">
                <div className="h-12 rounded-lg border border-border bg-surface-2" />
                <div className="flex items-end justify-between">
                  <div className="h-9 w-40 rounded bg-surface-2" />
                  <div className="h-9 w-20 rounded bg-surface-2" />
                </div>
                <div className="space-y-2">
                  {GUARDRAILS.map((g) => (
                    <div
                      key={g.key}
                      className="h-10 rounded-md border border-border bg-surface-2"
                    />
                  ))}
                </div>
              </div>
              <div className="mt-5 flex items-center justify-center gap-2.5 font-mono text-[12px] text-text-muted">
                <span className="inline-block h-2 w-2 animate-ping rounded-full bg-accent" />
                querying {selected.label} on testnet…
              </div>
            </motion.div>
          )}

          {result?.unavailable && (
            <motion.div
              key="unavail"
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="flex h-64 flex-col items-center justify-center text-center"
            >
              <div className="font-mono text-lg text-text-muted">
                feed unavailable
              </div>
              <div className="mt-2 max-w-xs text-[13px] leading-relaxed text-text-dim">
                {selected.label} is not deployed at its published testnet
                address. No price is shown — we don&apos;t fabricate one.
              </div>
            </motion.div>
          )}

          {result?.error && (
            <motion.div
              key="err"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="flex h-64 flex-col items-center justify-center text-center"
            >
              <div className="font-mono text-sm text-danger">
                live query failed
              </div>
              <div className="mt-2 max-w-xs text-[12px] text-text-dim">
                The testnet RPC didn&apos;t answer. Try again — nothing is
                cached or faked.
              </div>
            </motion.div>
          )}

          {(approved || rejected) && (
            <motion.div
              key="verdict"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.5, ease: EASE }}
            >
              {/* Verdict banner */}
              <div
                className="flex items-baseline gap-3 rounded-lg border px-4 py-3"
                style={{
                  borderColor: approved
                    ? "rgba(0,255,148,0.4)"
                    : "rgba(255,45,85,0.4)",
                  background: approved
                    ? "rgba(0,255,148,0.06)"
                    : "rgba(255,45,85,0.06)",
                }}
              >
                <span
                  className="font-mono text-xl font-semibold tracking-tight"
                  style={{ color: approved ? "#00ff94" : "#ff2d55" }}
                >
                  {approved ? "APPROVED" : "REJECTED"}
                </span>
                <span className="font-mono text-sm text-text-muted">
                  {asset}/USD · {selected.label}
                </span>
              </div>

              {/* Price */}
              <div className="mt-5 flex items-end justify-between">
                <div>
                  <div className="text-[11px] uppercase tracking-wider text-text-dim">
                    {approved ? "Validated price" : "Price withheld"}
                  </div>
                  <div className="tabular mt-1 font-mono text-3xl text-text">
                    {approved ? formatPrice14(result!.price!) : "—"}
                  </div>
                </div>
                {result?.timestamp ? (
                  <DataAge timestamp={result.timestamp} />
                ) : null}
              </div>

              {/* Guardrail row */}
              <div className="mt-6 space-y-2">
                {GUARDRAILS.map((g, i) => {
                  const isFail = g.key === failKey;
                  const reached = approved || failIdx === -1 || i <= failIdx;
                  const state = isFail ? "fail" : reached ? "pass" : "skip";
                  return (
                    <motion.div
                      key={g.key}
                      initial={{ opacity: 0, x: -6 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: 0.12 * i + 0.15, ease: EASE }}
                      className="flex items-center justify-between rounded-md border border-border px-3.5 py-2.5"
                    >
                      <span className="font-mono text-[13px] text-text-muted">
                        {g.label}
                      </span>
                      <span
                        className="font-mono text-[13px]"
                        style={{
                          color:
                            state === "fail"
                              ? "#ff2d55"
                              : state === "pass"
                                ? "#00ff94"
                                : "#7a7a82",
                        }}
                      >
                        {state === "fail"
                          ? "✗ blocked"
                          : state === "pass"
                            ? "✓ pass"
                            : "— not reached"}
                      </span>
                    </motion.div>
                  );
                })}
              </div>

              {/* Reason */}
              {rejected && result?.violation != null && (
                <div className="mt-5 rounded-lg border border-border bg-surface px-4 py-3">
                  <span className="font-mono text-[13px] text-danger">
                    {VIOLATIONS[result.violation]?.name ?? "Violation"}
                  </span>
                  <p className="mt-1 text-[12px] leading-relaxed text-text-muted">
                    {VIOLATIONS[result.violation]?.blurb}
                  </p>
                </div>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
