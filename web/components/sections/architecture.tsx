"use client";

import { useState, useRef, useEffect } from "react";
import { motion, useInView } from "framer-motion";
import { SectionShell } from "./section-shell";
import { useReducedMotion } from "@/lib/use-reduced-motion";

type ScenarioId = "happy" | "spike" | "thin" | "stale";

interface Scenario {
  id: ScenarioId;
  label: string;
  shortLabel: string;
  result: "ok" | "err";
  errorType?: string;
  rejectStage?: "layer1-deviation" | "layer1-staleness" | "layer2-liquidity";
  txHash?: string; // undefined = mock
  txHashShort?: string;
  ledger?: string;
  latencyMs: number;
  description: string;
}

const SCENARIOS: Scenario[] = [
  {
    id: "happy",
    label: "Happy Path — successful borrow",
    shortLabel: "Happy Path",
    result: "ok",
    txHash:
      "ce481203" + "1daa61ecb987c45123fbaba52eb83fe0b27f623dd3fa3fa0ec8a5c45",
    txHashShort: "ce481203...5c45",
    ledger: "2,450,314",
    latencyMs: 3200,
    description:
      "All five guards pass with Layer 2 enabled. Oracle price validated. Borrow approved.",
  },
  {
    id: "spike",
    label: "10× Price Spike — adversarial replay",
    shortLabel: "10× Spike",
    result: "err",
    errorType: "ExcessiveDeviation",
    rejectStage: "layer1-deviation",
    txHash:
      "a1cfdec1" + "fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653",
    txHashShort: "a1cfdec1...7653",
    latencyMs: 1800,
    description:
      "Reflector reports 10× price spike. Layer 1 catches the deviation. Borrow rejected.",
  },
  {
    id: "thin",
    label: "Thin Liquidity Attack",
    shortLabel: "Thin Liquidity",
    result: "err",
    errorType: "InsufficientLiquidity",
    rejectStage: "layer2-liquidity",
    latencyMs: 2400,
    description:
      "30m SDEX volume below threshold. Layer 2 (opt-in) rejects. Real-world testnet replay coming.",
  },
  {
    id: "stale",
    label: "Stale Oracle",
    shortLabel: "Stale Oracle",
    result: "err",
    errorType: "StaleData",
    rejectStage: "layer1-staleness",
    txHash:
      "7b799e02" + "c54d90334e2c45a2acdf2c43f4652d1fb125073896ebce1dc72a21f9",
    txHashShort: "7b799e02...21f9",
    latencyMs: 1500,
    description:
      "Oracle reading carried a 48h-old timestamp, past the staleness window. Layer 1 rejects with StaleData.",
  },
];

type NodeState = "idle" | "active" | "passed" | "failed";

interface DiagramState {
  user: NodeState;
  lending: NodeState;
  safeOracle: NodeState;
  layer1: NodeState;
  layer2: NodeState;
  cb: NodeState;
  result: NodeState;
}

const INITIAL_STATE: DiagramState = {
  user: "idle",
  lending: "idle",
  safeOracle: "idle",
  layer1: "idle",
  layer2: "idle",
  cb: "idle",
  result: "idle",
};

export function Architecture() {
  const [activeScenario, setActiveScenario] = useState<ScenarioId | null>(null);
  const [state, setState] = useState<DiagramState>(INITIAL_STATE);
  const [showResult, setShowResult] = useState(false);
  // Tracks whether a timeline is currently in flight — covers both the
  // first run AND every premium replay. Drives the connector pulses so
  // they stay alive across the continuous loop.
  const [isAnimating, setIsAnimating] = useState(false);
  const reducedMotion = useReducedMotion();

  // Pass 5B: Happy Path auto-runs once when the section first enters
  // the viewport. Replaces the dead "Pick a scenario" placeholder so
  // the most-used presentation slide is alive on first sight. Gated
  // by `hasAutoRun` so navigating back to the slide doesn't restart.
  // The ref hangs on the title (which IS a real element with a box —
  // `display: contents` wrappers are invisible to IntersectionObserver).
  const titleRef = useRef<HTMLHeadingElement>(null);
  const inView = useInView(titleRef, { once: true, margin: "-100px" });
  const [hasAutoRun, setHasAutoRun] = useState(false);

  // Every setTimeout from a run is registered here so it can be torn
  // down before a new run, on reset, and on unmount. Without this,
  // rapid scenario switching overlaps timelines (diagram settles on a
  // stale state), reset is fought by queued timers, and navigating the
  // deck away mid-run leaks timers / setStates into an inert component.
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([]);
  // Separate handle for the post-completion "premium hold → replay"
  // timer. Kept apart from timersRef so it survives the timeline
  // cleanup that fires when a fresh replay starts.
  const replayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearAllTimers = () => {
    timersRef.current.forEach(clearTimeout);
    timersRef.current = [];
    if (replayTimerRef.current) {
      clearTimeout(replayTimerRef.current);
      replayTimerRef.current = null;
    }
  };

  useEffect(() => clearAllTimers, []);

  // Continuous premium loop. After a scenario settles, the right-side
  // result panel (tx hash, ledger, latency, description) stays mounted
  // and untouched — only the left diagram quietly replays so the
  // section never feels frozen. Replay is suppressed under
  // `prefers-reduced-motion`.
  const runScenario = (id: ScenarioId, opts: { replay?: boolean } = {}) => {
    const isReplay = opts.replay === true;
    // Cancel any in-flight timeline before starting a new one.
    clearAllTimers();
    const scenario = SCENARIOS.find((s) => s.id === id)!;
    if (!isReplay) {
      setActiveScenario(id);
      setShowResult(false);
    }
    setState(INITIAL_STATE);
    setIsAnimating(true);

    // Sequence: user → lending → safe-oracle → layer1 → layer2 → cb → result
    const timeline: Array<[number, Partial<DiagramState>]> = [
      [100, { user: "active" }],
      [500, { user: "passed", lending: "active" }],
      [1000, { lending: "passed", safeOracle: "active" }],
      [1500, { layer1: "active" }],
    ];

    if (
      scenario.rejectStage === "layer1-deviation" ||
      scenario.rejectStage === "layer1-staleness"
    ) {
      timeline.push([2300, { layer1: "failed" }]);
    } else {
      timeline.push([2300, { layer1: "passed", layer2: "active" }]);

      if (scenario.rejectStage === "layer2-liquidity") {
        timeline.push([3100, { layer2: "failed" }]);
      } else {
        timeline.push([3100, { layer2: "passed", cb: "active" }]);
        timeline.push([
          3700,
          {
            cb: "passed",
            result: scenario.result === "ok" ? "passed" : "failed",
          },
        ]);
      }
    }

    if (scenario.rejectStage) {
      const lastIdx = timeline.length - 1;
      timeline[lastIdx] = [
        timeline[lastIdx][0],
        { ...timeline[lastIdx][1], result: "failed" },
      ];
    }

    timeline.forEach(([delay, change]) => {
      timersRef.current.push(
        setTimeout(() => {
          setState((prev) => ({ ...prev, ...change }));
        }, delay),
      );
    });

    timersRef.current.push(
      setTimeout(() => {
        if (!isReplay) setShowResult(true);
        setIsAnimating(false);
        // Premium hold: let the final lit state breathe for a calm
        // beat, then quietly restart the diagram. The result panel
        // is never touched, so tx hash / ledger / latency never
        // flicker. CSS `transition-colors` on each box handles the
        // soft fade back to idle when INITIAL_STATE is set.
        if (!reducedMotion) {
          replayTimerRef.current = setTimeout(() => {
            runScenario(id, { replay: true });
          }, 3200);
        }
      }, scenario.latencyMs + 200),
    );
  };

  const reset = () => {
    clearAllTimers();
    setActiveScenario(null);
    setState(INITIAL_STATE);
    setShowResult(false);
    setIsAnimating(false);
  };

  // Auto-run trigger. We deliberately omit `runScenario` from the
  // deps — it's recreated every render and would refire the effect;
  // hasAutoRun is the real one-shot gate.
  useEffect(() => {
    if (inView && !hasAutoRun) {
      setHasAutoRun(true);
      runScenario("happy");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inView, hasAutoRun]);

  const currentScenario = activeScenario
    ? SCENARIOS.find((s) => s.id === activeScenario)
    : null;

  // Pulse animation gate: true only while a timeline is actively
  // running (covers both the first run and every premium replay).
  // Goes false during the calm hold between cycles, which keeps the
  // Pass 5B/4F finite-pulse contract — pulses stop when nothing is
  // in flight, so we never regress into "infinite animation forever".
  const isRunning = isAnimating;

  return (
    <SectionShell id="architecture" eyebrow="Architecture" density="compact">
      <motion.h2
        ref={titleRef}
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7 }}
        className="t-h2"
      >
        Purely defensive.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.15, duration: 0.7 }}
        className="mt-4 max-w-xl text-text-muted"
      >
        Run a scenario and watch a borrow request flow through every guard —
        three trustless by default, plus Layer 2 when enabled — validated
        before it ever reaches your business logic.
      </motion.p>

      {/* Compact two-column console — fits one viewport, result always
          visible (no off-screen expand). Logic/state machine unchanged. */}
      <div className="mt-7 grid gap-5 lg:grid-cols-12">
        {/* Flow diagram */}
        <div className="surface-card p-5 md:p-6 lg:col-span-7">
          <div className="mb-5 flex items-center justify-between gap-4">
            <span className="font-mono text-[11px] uppercase tracking-[0.2em] text-text-dim">
              Request flow
            </span>
            <Legend />
          </div>
          <Flow state={state} isRunning={isRunning} />
        </div>

        {/* Console: scenarios + always-visible result */}
        <div className="surface-card flex flex-col p-5 md:p-6 lg:col-span-5">
          <div className="flex items-center justify-between">
            <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-text-dim">
              Run scenario
            </div>
            {activeScenario && (
              <button
                type="button"
                onClick={reset}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-md border border-border/60 bg-surface/30 px-3 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-text-muted transition-colors hover:border-accent/50 hover:bg-surface/50 hover:text-accent"
              >
                Reset
              </button>
            )}
          </div>

          <div className="mt-4 grid grid-cols-2 gap-2">
            {SCENARIOS.map((s) => {
              const isActive = activeScenario === s.id;
              const outcomeBadge = s.result === "ok" ? "pass" : "reject";
              return (
                <button
                  key={s.id}
                  onClick={() => runScenario(s.id)}
                  className={`group flex cursor-pointer items-center justify-between gap-2 rounded-lg border px-3 py-3 text-left transition-all ${
                    isActive
                      ? "border-accent bg-accent/10"
                      : "border-border bg-surface/20 hover:-translate-y-0.5 hover:border-accent/60 hover:bg-surface/50 active:translate-y-0"
                  }`}
                >
                  <span
                    className={`font-mono text-xs font-medium ${
                      isActive ? "text-accent" : "text-text"
                    }`}
                  >
                    {s.shortLabel}
                  </span>
                  {isActive ? (
                    <span className="flex items-center gap-1 font-mono text-[9px] uppercase tracking-[0.18em] text-accent">
                      <span className="h-1.5 w-1.5 rounded-full bg-accent" />
                      Active
                    </span>
                  ) : (
                    <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-text-muted">
                      {outcomeBadge}
                    </span>
                  )}
                </button>
              );
            })}
          </div>

          {/* Result — reserved space so it never pushes layout / off-screen */}
          <div className="mt-4 min-h-[190px] overflow-hidden rounded-xl border border-border bg-[var(--color-background)]">
            {showResult && currentScenario ? (
              <motion.div
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.3 }}
              >
                {/* Outcome banner */}
                <div
                  className={`flex items-center gap-2.5 border-b px-4 py-3 font-mono text-sm ${
                    currentScenario.result === "ok"
                      ? "border-accent/20 bg-accent/10 text-accent"
                      : "border-danger/20 bg-danger/10 text-danger"
                  }`}
                >
                  <span className="text-base leading-none">
                    {currentScenario.result === "ok" ? "✓" : "✗"}
                  </span>
                  <span className="font-medium">
                    {currentScenario.result === "ok"
                      ? "Ok(price)"
                      : `Err(${currentScenario.errorType})`}
                  </span>
                  <span className="ml-auto text-[10px] uppercase tracking-[0.18em] opacity-70">
                    {currentScenario.result === "ok" ? "validated" : "rejected"}
                  </span>
                </div>

                <div className="space-y-3 p-4">
                  <p className="text-sm leading-relaxed text-text-muted">
                    {currentScenario.description}
                  </p>

                  {/* Metadata grid — labelled rows */}
                  <dl className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 border-t border-border pt-3 font-mono text-xs">
                    <dt className="text-text-dim">Transaction</dt>
                    <dd className="text-right">
                      {currentScenario.txHash ? (
                        <a
                          href={`https://stellar.expert/explorer/testnet/tx/${currentScenario.txHash}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-text transition-colors hover:text-accent"
                        >
                          {currentScenario.txHashShort} ↗
                        </a>
                      ) : (
                        <span className="text-text-dim">
                          simulated — replay coming
                        </span>
                      )}
                    </dd>

                    {currentScenario.ledger && (
                      <>
                        <dt className="text-text-dim">Ledger</dt>
                        <dd className="text-right text-text-muted">
                          {currentScenario.ledger}
                        </dd>
                      </>
                    )}

                    <dt className="text-text-dim">Latency</dt>
                    <dd className="text-right text-text-muted">
                      ~{(currentScenario.latencyMs / 1000).toFixed(1)}s
                    </dd>
                  </dl>
                </div>
              </motion.div>
            ) : (
              <div className="flex min-h-[190px] flex-col items-center justify-center gap-2 px-4 text-center">
                <span
                  className={`font-mono text-xs uppercase tracking-[0.2em] ${
                    activeScenario ? "text-accent" : "text-text-dim"
                  }`}
                >
                  {activeScenario ? "Validating…" : "Idle"}
                </span>
                <span className="text-xs text-text-dim">
                  {activeScenario
                    ? "Running guards in sequence"
                    : "Pick a scenario to run the guards"}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>
    </SectionShell>
  );
}

// =====================================================================
// Vertical request-flow visual — symmetric connectors with a
// travelling data-pulse. State-driven colour only; the scenario state
// machine and data above are unchanged.
// =====================================================================

const STATE_KEY: { k: string; label: string; cls: string }[] = [
  { k: "idle", label: "idle", cls: "bg-border" },
  { k: "active", label: "active", cls: "bg-accent" },
  { k: "passed", label: "pass", cls: "bg-accent/50" },
  { k: "failed", label: "fail", cls: "bg-danger" },
];

function Legend() {
  return (
    <div className="hidden items-center gap-3.5 font-mono text-[10px] uppercase tracking-[0.14em] text-text-dim sm:flex">
      {STATE_KEY.map((s) => (
        <span key={s.k} className="flex items-center gap-1.5">
          <span className={`h-1.5 w-1.5 rounded-full ${s.cls}`} />
          {s.label}
        </span>
      ))}
    </div>
  );
}

function tone(s: NodeState) {
  switch (s) {
    case "active":
      return "border-accent bg-accent/10 text-accent";
    case "passed":
      return "border-accent/50 bg-accent/5 text-accent";
    case "failed":
      return "border-danger bg-danger/10 text-danger";
    default:
      return "border-border bg-[var(--color-background)] text-text-muted";
  }
}

function glow(s: NodeState): string | undefined {
  if (s === "active" || s === "passed")
    return "0 0 22px -6px rgba(0,255,148,0.45)";
  if (s === "failed") return "0 0 22px -6px rgba(255,45,85,0.45)";
  return undefined;
}

function Flow({
  state,
  isRunning,
}: {
  state: DiagramState;
  isRunning: boolean;
}) {
  return (
    <div className="mx-auto flex w-full max-w-[20rem] flex-col items-stretch font-mono">
      <Box label="USER" state={state.user} />
      <Connector
        active={state.user !== "idle"}
        isRunning={isRunning}
        label="borrow()"
      />
      <Box label="mock-lending" state={state.lending} />
      <Connector
        active={state.lending !== "idle"}
        isRunning={isRunning}
        label="safe_oracle::lastprice()"
      />

      {/* safe-oracle library — the three guards */}
      <motion.div
        animate={{
          boxShadow:
            state.safeOracle === "idle"
              ? "0 0 0 0 rgba(0,0,0,0)"
              : "0 0 32px -12px rgba(0,255,148,0.4)",
        }}
        transition={{ duration: 0.4 }}
        className={`rounded-xl border p-4 transition-colors ${
          state.safeOracle === "idle"
            ? "border-border"
            : "border-accent/30 bg-accent/5"
        }`}
      >
        <div className="mb-3 text-center text-[10px] uppercase tracking-[0.2em] text-text-dim">
          safe-oracle
        </div>
        <div className="grid grid-cols-3 gap-2">
          <Guard label="Layer 1" sub="Default" state={state.layer1} />
          <Guard label="Layer 2" sub="Opt-in" state={state.layer2} />
          <Guard label="Breaker" sub="Auto-halt" state={state.cb} />
        </div>
      </motion.div>

      <Connector
        active={state.result !== "idle"}
        isRunning={isRunning}
        failed={state.result === "failed"}
      />

      {/* Result chip */}
      <div className="flex justify-center">
        <motion.div
          key={state.result}
          initial={
            state.result !== "idle"
              ? { scale: 0.85, opacity: 0 }
              : { scale: 1, opacity: 1 }
          }
          animate={{ scale: 1, opacity: 1 }}
          transition={{ type: "spring", stiffness: 220, damping: 16 }}
          className={`rounded-lg border px-5 py-2.5 text-sm ${
            state.result === "passed"
              ? "border-accent bg-accent/10 text-accent"
              : state.result === "failed"
              ? "border-danger bg-danger/10 text-danger"
              : "border-border text-text-dim"
          }`}
          style={{ boxShadow: glow(state.result) }}
        >
          {state.result === "passed"
            ? "✓ Ok(price)"
            : state.result === "failed"
            ? "✗ Err(violation)"
            : "awaiting result"}
        </motion.div>
      </div>
    </div>
  );
}

function Box({ label, state }: { label: string; state: NodeState }) {
  return (
    <motion.div
      animate={{ scale: state === "active" ? 1.02 : 1 }}
      transition={{ duration: 0.3 }}
      className={`relative w-full overflow-hidden rounded-lg border px-4 py-3 text-center text-sm transition-colors ${tone(
        state
      )}`}
      style={{ boxShadow: glow(state) }}
    >
      {/* lit top hairline — premium edge */}
      <span
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-px"
        style={{
          background:
            "linear-gradient(90deg, transparent, rgba(255,255,255,0.12), transparent)",
        }}
      />
      {label}
    </motion.div>
  );
}

function Guard({
  label,
  sub,
  state,
}: {
  label: string;
  sub: string;
  state: NodeState;
}) {
  return (
    <motion.div
      animate={{ scale: state === "active" ? 1.045 : 1 }}
      transition={{ duration: 0.3 }}
      className={`rounded-lg border px-2 py-3 text-center transition-colors ${tone(
        state
      )}`}
      style={{ boxShadow: glow(state) }}
    >
      <div className="text-xs font-medium">{label}</div>
      <div className="mt-0.5 text-[10px] opacity-70">{sub}</div>
    </motion.div>
  );
}

/**
 * Symmetric connector — a centred vertical line, a centred arrowhead,
 * and an optional centred caption (everything on the same vertical
 * axis, no side-offset asymmetry). When the segment is energised a
 * glowing data-pulse travels down the line — that motion IS the
 * "request flowing through the guards".
 */
function Connector({
  active,
  isRunning,
  failed = false,
  label,
}: {
  active: boolean;
  isRunning: boolean;
  failed?: boolean;
  label?: string;
}) {
  const reducedMotion = useReducedMotion();
  const lit = active && !failed;
  const lineColor = !active
    ? "var(--color-border-strong)"
    : failed
    ? "var(--color-danger)"
    : "var(--color-accent)";

  return (
    <div className="flex flex-col items-center gap-1 py-1">
      <span
        className="relative h-7 w-px"
        style={{ background: "var(--color-border)" }}
      >
        <motion.span
          className="absolute inset-x-0 top-0 h-full origin-top"
          style={{ background: lineColor }}
          initial={{ scaleY: 0 }}
          animate={{ scaleY: active ? 1 : 0 }}
          transition={{ duration: 0.35 }}
        />
        {/* Travelling data-pulse — only animates while the scenario
            is in flight (Pass 5B finite-pulse gate). `isRunning` flips
            false the moment `showResult` becomes true, so the pulse
            stops on its own and React unmounts the motion.span; zero
            ongoing cost matches the Pass 4B perf model.

            Pass 7 visibility bump: dot size 2 → 2.5 (8 → 10 px),
            stronger glow (10 px → 14 px spread, 0.65 → 0.8 alpha),
            and a hair faster cadence (0.85 s → 0.7 s) so each pulse
            reads as a clearly energetic data-packet rather than a
            subtle drift. The pulse path and finite-gate behaviour are
            unchanged. */}
        {lit && isRunning && !reducedMotion && (
          <motion.span
            className="absolute left-1/2 top-0 h-2.5 w-2.5 rounded-full"
            style={{
              background: "var(--color-accent)",
              boxShadow: "0 0 14px 3px rgba(0,255,148,0.8)",
            }}
            initial={{ x: "-50%", y: -8, opacity: 0 }}
            animate={{ x: "-50%", y: [-8, 28], opacity: [0, 1, 1, 0] }}
            transition={{ duration: 0.7, repeat: Infinity, ease: "easeIn" }}
          />
        )}
      </span>

      <motion.span
        animate={{ opacity: active ? 1 : 0.4 }}
        transition={{ duration: 0.3 }}
        style={{ color: lineColor }}
        aria-hidden="true"
        className="block leading-none"
      >
        <svg width="12" height="7" viewBox="0 0 12 7" fill="none">
          <path
            d="M1 1L6 5.5L11 1"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </motion.span>

      {label && (
        <span className="mt-0.5 max-w-full text-center text-[10px] tracking-wide text-text-dim">
          {label}
        </span>
      )}
    </div>
  );
}
