"use client";

import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { SectionShell } from "./section-shell";

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
      "All five guardrails pass. Oracle price validated. Borrow approved.",
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
      "30m SDEX volume below threshold. Layer 2 rejects. Real-world testnet replay coming.",
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

  const runScenario = (id: ScenarioId) => {
    const scenario = SCENARIOS.find((s) => s.id === id)!;
    setActiveScenario(id);
    setShowResult(false);
    setState(INITIAL_STATE);

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
      setTimeout(() => {
        setState((prev) => ({ ...prev, ...change }));
      }, delay);
    });

    setTimeout(() => setShowResult(true), scenario.latencyMs + 200);
  };

  const reset = () => {
    setActiveScenario(null);
    setState(INITIAL_STATE);
    setShowResult(false);
  };

  const currentScenario = activeScenario
    ? SCENARIOS.find((s) => s.id === activeScenario)
    : null;

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
        Watch a borrow request flow through five guards. Run any scenario to see
        how safe-oracle validates the oracle response before reaching your
        business logic.
      </motion.p>

      {/* Diagram */}
      <motion.div
        initial={{ opacity: 0, y: 30 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-50px" }}
        transition={{ delay: 0.3, duration: 0.8 }}
        className="mt-16 rounded-xl border border-border bg-surface p-8 md:p-12"
      >
        <Diagram state={state} />
      </motion.div>

      {/* Scenario controls */}
      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.5, duration: 0.6 }}
        className="mt-8 rounded-xl border border-border bg-surface p-6"
      >
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div>
            <div className="font-mono text-xs uppercase tracking-wider text-text-muted">
              Run scenario
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            {SCENARIOS.map((s) => (
              <button
                key={s.id}
                onClick={() => runScenario(s.id)}
                disabled={activeScenario === s.id}
                className={`rounded-lg border px-4 py-2 font-mono text-sm transition-all ${
                  activeScenario === s.id
                    ? "border-accent/40 bg-accent/10 text-accent"
                    : "border-border text-text-muted hover:border-text-muted hover:text-text"
                } disabled:cursor-default`}
              >
                {s.shortLabel}
              </button>
            ))}
            {activeScenario && (
              <button
                onClick={reset}
                className="rounded-lg border border-border px-4 py-2 font-mono text-sm text-text-dim transition-all hover:border-text-muted hover:text-text-muted"
              >
                Reset
              </button>
            )}
          </div>
        </div>

        {/* Result panel */}
        <AnimatePresence>
          {showResult && currentScenario && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              transition={{ duration: 0.3 }}
              className="mt-6 overflow-hidden"
            >
              <div className="space-y-3 border-t border-border pt-6">
                <div className="flex items-start gap-3">
                  <span className="mt-1 font-mono text-xs text-text-dim">
                    RESULT
                  </span>
                  {currentScenario.result === "ok" ? (
                    <div className="flex items-center gap-2">
                      <span className="rounded bg-accent/10 px-2 py-1 font-mono text-xs text-accent">
                        ✓ Ok(price)
                      </span>
                      <span className="text-sm text-text-muted">
                        All guardrails passed
                      </span>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2">
                      <span className="rounded bg-danger/10 px-2 py-1 font-mono text-xs text-danger">
                        ✗ Err({currentScenario.errorType})
                      </span>
                      <span className="text-sm text-text-muted">
                        Borrow rejected
                      </span>
                    </div>
                  )}
                </div>

                <div className="flex items-start gap-3">
                  <span className="mt-1 font-mono text-xs text-text-dim">
                    DESC
                  </span>
                  <span className="text-sm text-text">
                    {currentScenario.description}
                  </span>
                </div>

                {currentScenario.txHash ? (
                  <div className="flex items-start gap-3">
                    <span className="mt-1 font-mono text-xs text-text-dim">
                      TX
                    </span>
                    <a
                      href={`https://stellar.expert/explorer/testnet/tx/${currentScenario.txHash}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 font-mono text-sm text-text-muted transition-colors hover:text-accent"
                    >
                      <span>{currentScenario.txHashShort}</span>
                      <span className="text-xs">↗</span>
                    </a>
                    {currentScenario.ledger && (
                      <span className="font-mono text-xs text-text-dim">
                        ledger {currentScenario.ledger}
                      </span>
                    )}
                  </div>
                ) : (
                  <div className="flex items-start gap-3">
                    <span className="mt-1 font-mono text-xs text-text-dim">
                      TX
                    </span>
                    <span className="rounded bg-text-dim/10 px-2 py-1 font-mono text-xs text-text-dim">
                      Simulated — testnet replay coming
                    </span>
                  </div>
                )}

                <div className="flex items-start gap-3">
                  <span className="mt-1 font-mono text-xs text-text-dim">
                    LATENCY
                  </span>
                  <span className="font-mono text-sm text-text-muted">
                    ~{(currentScenario.latencyMs / 1000).toFixed(1)}s
                  </span>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>
    </SectionShell>
  );
}

// =====================================================================
// SVG Diagram Component
// =====================================================================

interface DiagramProps {
  state: DiagramState;
}

function Diagram({ state }: DiagramProps) {
  const nodeColor = (s: NodeState): string => {
    switch (s) {
      case "idle":
        return "border-border bg-background text-text-muted";
      case "active":
        return "border-accent bg-accent/10 text-accent";
      case "passed":
        return "border-accent/60 bg-accent/5 text-accent";
      case "failed":
        return "border-danger bg-danger/10 text-danger";
    }
  };

  const showLine = (from: NodeState, to: NodeState): boolean => {
    return (
      from !== "idle" && (to === "active" || to === "passed" || to === "failed")
    );
  };

  return (
    <div className="font-mono">
      {/* User node */}
      <div className="flex justify-center">
        <Node
          label="USER"
          state={state.user}
          className={nodeColor(state.user)}
        />
      </div>

      {/* User → Lending */}
      <Connector
        to={state.lending}
        label="borrow()"
        active={showLine(state.user, state.lending)}
      />

      {/* Lending node */}
      <div className="flex justify-center">
        <Node
          label="mock-lending"
          state={state.lending}
          className={nodeColor(state.lending)}
          wide
        />
      </div>

      {/* Lending → safe-oracle */}
      <Connector
        to={state.safeOracle}
        label="safe_oracle::lastprice()"
        active={showLine(state.lending, state.safeOracle)}
      />

      {/* safe-oracle library box (contains layer1, layer2, cb) */}
      <div className="flex justify-center">
        <div
          className={`rounded-lg border-2 p-6 ${
            state.safeOracle === "idle"
              ? "border-border bg-background"
              : "border-accent/30 bg-accent/5"
          }`}
        >
          <div className="mb-4 text-center font-mono text-xs uppercase tracking-wider text-text-muted">
            safe-oracle library
          </div>
          <div className="flex flex-col items-center gap-4 md:flex-row md:items-stretch">
            <SubNode
              label="Layer 1"
              sublabel="Oracle checks"
              state={state.layer1}
            />
            <Arrow active={state.layer1 === "passed"} />
            <SubNode
              label="Layer 2"
              sublabel="Market structure"
              state={state.layer2}
            />
            <Arrow active={state.layer2 === "passed"} />
            <SubNode label="CB" sublabel="Circuit breaker" state={state.cb} />
          </div>
        </div>
      </div>

      {/* safe-oracle → result */}
      <Connector
        to={state.result}
        active={state.result !== "idle"}
        failed={state.result === "failed"}
      />

      {/* Result */}
      <div className="flex justify-center">
        {state.result === "passed" && (
          <motion.div
            initial={{ scale: 0.8, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ type: "spring", stiffness: 200, damping: 15 }}
            className="rounded-lg border-2 border-accent bg-accent/10 px-6 py-3 font-mono text-sm text-accent"
          >
            ✓ Ok(price)
          </motion.div>
        )}
        {state.result === "failed" && (
          <motion.div
            initial={{ scale: 0.8, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ type: "spring", stiffness: 200, damping: 15 }}
            className="rounded-lg border-2 border-danger bg-danger/10 px-6 py-3 font-mono text-sm text-danger"
          >
            ✗ Err(violation)
          </motion.div>
        )}
      </div>
    </div>
  );
}

function Node({
  label,
  state,
  className,
  wide = false,
}: {
  label: string;
  state: NodeState;
  className: string;
  wide?: boolean;
}) {
  return (
    <motion.div
      animate={{
        scale: state === "active" ? 1.02 : 1,
      }}
      transition={{ duration: 0.3 }}
      className={`rounded-lg border-2 px-4 py-3 text-center text-sm transition-all ${
        wide ? "min-w-[180px]" : "min-w-[100px]"
      } ${className}`}
      style={
        state === "active"
          ? {
              boxShadow: "0 0 24px rgba(0, 255, 148, 0.3)",
            }
          : state === "failed"
          ? {
              boxShadow: "0 0 24px rgba(255, 45, 85, 0.3)",
            }
          : {}
      }
    >
      {label}
    </motion.div>
  );
}

function SubNode({
  label,
  sublabel,
  state,
}: {
  label: string;
  sublabel: string;
  state: NodeState;
}) {
  const colorClasses =
    state === "idle"
      ? "border-border bg-background text-text-muted"
      : state === "active"
      ? "border-accent bg-accent/10 text-accent"
      : state === "passed"
      ? "border-accent/60 bg-accent/5 text-accent"
      : "border-danger bg-danger/10 text-danger";

  return (
    <motion.div
      animate={{
        scale: state === "active" ? 1.05 : 1,
      }}
      transition={{ duration: 0.3 }}
      className={`flex-1 rounded-lg border-2 px-4 py-3 text-center transition-all ${colorClasses}`}
      style={
        state === "active"
          ? { boxShadow: "0 0 20px rgba(0, 255, 148, 0.4)" }
          : state === "failed"
          ? { boxShadow: "0 0 20px rgba(255, 45, 85, 0.4)" }
          : {}
      }
    >
      <div className="text-sm font-medium">{label}</div>
      <div className="mt-1 text-xs opacity-70">{sublabel}</div>
    </motion.div>
  );
}

function Connector({
  to,
  label,
  active,
  failed = false,
}: {
  to: NodeState;
  label?: string;
  active: boolean;
  failed?: boolean;
}) {
  const isFail = failed || to === "failed";
  const barColor = active
    ? isFail
      ? "bg-danger"
      : "bg-accent"
    : "bg-border";

  return (
    <div className="my-4 flex flex-col items-center gap-1">
      <motion.div
        initial={{ height: 0 }}
        animate={{ height: active ? 24 : 0 }}
        transition={{ duration: 0.4 }}
        className={`w-px ${barColor}`}
      />
      {label && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: active ? 1 : 0.3 }}
          transition={{ duration: 0.3 }}
          className="text-xs text-text-dim"
        >
          {label}
        </motion.div>
      )}
      <motion.div
        initial={{ height: 0 }}
        animate={{ height: active ? 12 : 0 }}
        transition={{ duration: 0.4, delay: 0.1 }}
        className={`w-px ${barColor}`}
      />
    </div>
  );
}

function Arrow({ active }: { active: boolean }) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: active ? 1 : 0.3 }}
      transition={{ duration: 0.3 }}
      className={`hidden self-center md:block ${
        active ? "text-accent" : "text-border"
      }`}
    >
      →
    </motion.div>
  );
}
