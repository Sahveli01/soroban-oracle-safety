"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";
import { EASE_OUT_EXPO } from "@/lib/easings";

/**
 * Operator — interactive sink preview.
 *
 * Pass 5B rewrite. The Pass 4A grid of envKey-hint cards was honest
 * but inert; user feedback asked for something demo-like ("Discord
 * veya Telegram bot mesaj örneği gibi olabilir"). The new layout is a
 * chip-list / preview pane: clicking a sink swaps the right pane to
 * a visual approximation of how the alert lands in that platform.
 *
 * The alert payload is identical across all six previews (same asset,
 * tx hash, deviation factor) — that's the demonstration: one alert
 * dispatched into many platform shapes. Numbers match the testnet
 * 10× spike scenario in architecture.tsx (txHashShort
 * "a1cfdec1...7653"); the asset symbol "USTRY" matches the Rust
 * integration tests in crates/safe-oracle/tests/integration.rs.
 *
 * Previews are visual approximations, not pixel-replicas — they evoke
 * each platform's design language (Discord dark embed with side-bar,
 * Telegram chat bubble, Slack Block Kit, PagerDuty incident card,
 * generic JSON, Rust trait impl) without lifting logos or asset
 * trademarks. State change on click is instant — no animation on
 * preview swap (Pass 4B/4F perf model: avoid animated DOM transitions
 * inside deck slides).
 */

type SinkId =
  | "discord"
  | "telegram"
  | "slack"
  | "pagerduty"
  | "generic"
  | "custom";

interface Sink {
  id: SinkId;
  name: string;
  description: string;
}

const SINKS: Sink[] = [
  {
    id: "discord",
    name: "Discord",
    description: "Webhook → channel, embed with side-bar accent",
  },
  {
    id: "telegram",
    name: "Telegram",
    description: "Bot sendMessage to channel or group",
  },
  {
    id: "slack",
    name: "Slack",
    description: "Block Kit message via incoming webhook",
  },
  {
    id: "pagerduty",
    name: "PagerDuty",
    description: "Events API v2 incident, dedup-keyed",
  },
  {
    id: "generic",
    name: "Generic Webhook",
    description: "POST JSON to any URL with custom headers",
  },
  {
    id: "custom",
    name: "Custom Sink",
    description: "Implement WebhookSink, register, deploy",
  },
];

// Shared payload — every preview renders the same logical alert in
// the host platform's shape. Numbers cross-reference architecture.tsx
// and the integration tests; do not drift these without updating both.
const ALERT = {
  asset: "USTRY/USDC",
  deviation: "10.2×",
  severity: "HIGH",
  action: "Borrow rejected",
  tx: "a1cfdec1...7653",
  ts: "14:32",
  ledger: "2,450,314",
};

export function Operator() {
  const [activeSink, setActiveSink] = useState<SinkId>("discord");

  return (
    <SectionShell id="operator" eyebrow="Operator" density="dense">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: EASE_OUT_EXPO }}
        className="t-h2"
      >
        Plug in your stack.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.15, duration: 0.7 }}
        className="mt-4 max-w-xl text-text-muted"
      >
        oracle-watch dispatches the same alert to every configured sink.
        Click a sink — see exactly how the alert lands there.
      </motion.p>

      <div className="mt-7 grid gap-5 lg:grid-cols-12">
        {/* Sink chips — left column on lg, top on mobile */}
        <div className="space-y-2 lg:col-span-5">
          {SINKS.map((s, i) => {
            const isActive = activeSink === s.id;
            return (
              <motion.button
                key={s.id}
                onClick={() => setActiveSink(s.id)}
                initial={{ opacity: 0, x: -8 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ delay: i * 0.04, duration: 0.4, ease: EASE_OUT_EXPO }}
                className={`group flex w-full cursor-pointer items-center justify-between gap-3 rounded-md border px-4 py-3 text-left transition-all ${
                  isActive
                    ? "border-accent bg-accent/10"
                    : "border-border/60 bg-surface/30 hover:translate-x-0.5 hover:border-accent/60 hover:bg-surface/50 active:translate-x-0"
                }`}
              >
                <div className="min-w-0">
                  <div
                    className={`font-mono text-sm font-medium ${
                      isActive ? "text-accent" : "text-text"
                    }`}
                  >
                    {s.name}
                  </div>
                  <div className="mt-0.5 text-[11px] text-text-muted">
                    {s.description}
                  </div>
                </div>
                {isActive ? (
                  <span className="flex flex-shrink-0 items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-accent">
                    <span className="h-1.5 w-1.5 rounded-full bg-accent" />
                    Active
                  </span>
                ) : (
                  <span className="flex-shrink-0 font-mono text-xs text-text-dim transition-colors group-hover:text-accent">
                    →
                  </span>
                )}
              </motion.button>
            );
          })}
        </div>

        {/* Preview pane — right column on lg */}
        <motion.div
          key={activeSink}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.18 }}
          className="lg:col-span-7"
        >
          <SinkPreview sink={activeSink} />
        </motion.div>
      </div>
    </SectionShell>
  );
}

function SinkPreview({ sink }: { sink: SinkId }) {
  switch (sink) {
    case "discord":
      return <DiscordPreview />;
    case "telegram":
      return <TelegramPreview />;
    case "slack":
      return <SlackPreview />;
    case "pagerduty":
      return <PagerDutyPreview />;
    case "generic":
      return <GenericPreview />;
    case "custom":
      return <CustomPreview />;
  }
}

/* ---------- Platform previews — visual approximations ---------- */

function DiscordPreview() {
  return (
    <div className="rounded-md bg-[#313338] p-4 ring-1 ring-white/10">
      <div className="mb-3 flex items-center gap-2 border-b border-white/5 pb-2">
        <span className="text-sm text-white/50">#</span>
        <span className="text-sm font-medium text-white">oracle-alerts</span>
      </div>
      <div className="flex gap-3">
        <div className="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full bg-accent/30 font-mono text-[10px] font-bold text-accent">
          SO
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-2">
            <span className="text-sm font-medium text-white">SAFE-ORACLE</span>
            <span className="rounded-sm bg-[#5865f2] px-1 py-0.5 text-[9px] font-bold uppercase text-white">
              App
            </span>
            <span className="text-[10px] text-white/40">Today at {ALERT.ts}</span>
          </div>
          <div className="mt-2 rounded border-l-4 border-amber-400 bg-[#2b2d31] p-3">
            <div className="text-sm font-semibold text-white">
              🚨 Excessive Deviation
            </div>
            <div className="mt-1.5 text-xs leading-relaxed text-white/80">
              {ALERT.asset} reported {ALERT.deviation} deviation. Borrow
              rejected; circuit breaker armed.
            </div>
            <div className="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-[10px]">
              <div>
                <div className="font-bold text-white/40">ASSET</div>
                <div className="mt-0.5 font-mono text-white">{ALERT.asset}</div>
              </div>
              <div>
                <div className="font-bold text-white/40">SEVERITY</div>
                <div className="mt-0.5 font-mono text-amber-400">
                  {ALERT.severity}
                </div>
              </div>
              <div className="col-span-2">
                <div className="font-bold text-white/40">TX</div>
                <div className="mt-0.5 truncate font-mono text-white">
                  {ALERT.tx}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function TelegramPreview() {
  return (
    <div className="rounded-md bg-[#17212b] p-4 ring-1 ring-white/5">
      <div className="mb-3 flex items-center gap-3 border-b border-white/5 pb-2">
        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-accent/30 font-mono text-[10px] font-bold text-accent">
          SO
        </div>
        <div>
          <div className="text-sm font-medium text-white">SAFE-ORACLE</div>
          <div className="text-[10px] text-white/50">bot</div>
        </div>
      </div>
      <div className="flex">
        <div className="max-w-md rounded-lg rounded-tl-sm bg-white/[0.06] p-3">
          <div className="whitespace-pre font-mono text-xs leading-relaxed text-white">
{`🚨 SAFE-ORACLE Alert

Asset:  ${ALERT.asset}
Event:  Excessive deviation (${ALERT.deviation})
Action: ${ALERT.action}
Tx:     ${ALERT.tx}`}
          </div>
          <div className="mt-2 text-right text-[9px] text-white/40">
            {ALERT.ts} ✓✓
          </div>
        </div>
      </div>
    </div>
  );
}

function SlackPreview() {
  return (
    <div className="rounded-md bg-[#1a1d21] p-4 ring-1 ring-white/10">
      <div className="mb-3 flex items-center gap-2 border-b border-white/5 pb-2">
        <div className="flex h-6 w-6 items-center justify-center rounded bg-accent/30 font-mono text-[9px] font-bold text-accent">
          SO
        </div>
        <span className="text-sm font-bold text-white">SAFE-ORACLE</span>
        <span className="rounded bg-white/10 px-1.5 py-0.5 text-[8px] font-bold uppercase text-white/70">
          App
        </span>
        <span className="ml-auto text-[10px] text-white/40">{ALERT.ts}</span>
      </div>
      <div className="text-lg font-bold leading-tight text-white">
        🚨 Excessive Deviation
      </div>
      <div className="mt-2 text-xs leading-relaxed text-white/80">
        Layer 1 deviation guard rejected a borrow on {ALERT.asset}. Reflector
        reported a {ALERT.deviation} deviation from baseline.
      </div>
      <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-2 text-[11px]">
        <div>
          <div className="mb-0.5 font-bold text-white">Asset</div>
          <div className="font-mono text-white/80">{ALERT.asset}</div>
        </div>
        <div>
          <div className="mb-0.5 font-bold text-white">Severity</div>
          <div className="font-mono text-amber-400">{ALERT.severity}</div>
        </div>
        <div>
          <div className="mb-0.5 font-bold text-white">Action</div>
          <div className="font-mono text-white/80">{ALERT.action}</div>
        </div>
        <div className="min-w-0">
          <div className="mb-0.5 font-bold text-white">Tx</div>
          <div className="truncate font-mono text-white/80">{ALERT.tx}</div>
        </div>
      </div>
    </div>
  );
}

function PagerDutyPreview() {
  return (
    <div className="rounded-md border border-red-500/30 bg-[#0f1419] p-4">
      <div className="mb-3 flex items-center justify-between border-b border-white/5 pb-2">
        <div className="flex items-center gap-2">
          <span className="rounded bg-red-600 px-2 py-0.5 font-mono text-[10px] font-bold text-white">
            P1
          </span>
          <span className="rounded border border-red-500/50 px-1.5 py-0.5 font-mono text-[9px] font-bold uppercase tracking-wider text-red-400">
            Triggered
          </span>
        </div>
        <span className="text-[10px] text-white/40">{ALERT.ts} UTC</span>
      </div>
      <div className="text-sm font-semibold text-white">
        Excessive Deviation — {ALERT.asset}
      </div>
      <div className="mt-1.5 text-xs leading-relaxed text-white/70">
        Reflector reported {ALERT.deviation} deviation from baseline. Auto-halt
        engaged. Manual review required before unhalting.
      </div>
      <div className="mt-3 space-y-1 border-t border-white/5 pt-3 text-[10px]">
        <div className="flex">
          <span className="w-28 font-mono uppercase tracking-wider text-white/40">
            Service
          </span>
          <span className="font-mono text-white/80">safe-oracle</span>
        </div>
        <div className="flex">
          <span className="w-28 font-mono uppercase tracking-wider text-white/40">
            Dedup Key
          </span>
          <span className="font-mono text-white/80">
            ustry-deviation-2026-05-21
          </span>
        </div>
        <div className="flex min-w-0">
          <span className="w-28 flex-shrink-0 font-mono uppercase tracking-wider text-white/40">
            Tx
          </span>
          <span className="truncate font-mono text-white/80">{ALERT.tx}</span>
        </div>
      </div>
    </div>
  );
}

function GenericPreview() {
  const payload = `{
  "event": "excessive_deviation",
  "asset": "${ALERT.asset}",
  "deviation_factor": 10.2,
  "severity": "high",
  "action": "borrow_rejected",
  "tx": "${ALERT.tx}",
  "ts": "2026-05-21T${ALERT.ts}:17Z",
  "source": "safe-oracle"
}`;
  return (
    <div className="rounded-md border border-border/60 bg-[var(--color-background)] p-4">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <div className="font-mono text-[10px] uppercase tracking-wider text-text-dim">
          POST /webhook
        </div>
        <span className="rounded bg-accent/10 px-2 py-0.5 font-mono text-[9px] text-accent">
          Content-Type: application/json
        </span>
      </div>
      <pre className="overflow-x-auto font-mono text-xs leading-relaxed text-text">
        {payload}
      </pre>
    </div>
  );
}

function CustomPreview() {
  const code = `impl WebhookSink for MyTeamSlackVariant {
    fn kind(&self) -> &str {
        "slack-myteam"
    }

    async fn send(&self, body: &str) -> Result<()> {
        let payload = self.wrap_for_my_team(body);
        self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}`;
  return (
    <div className="rounded-md border border-border/60 bg-[var(--color-background)] p-4">
      <div className="mb-2 font-mono text-[10px] uppercase tracking-wider text-text-dim">
        WebhookSink — implement, register in build_sinks(), deploy
      </div>
      <pre className="overflow-x-auto font-mono text-xs leading-relaxed text-text-muted">
        {code}
      </pre>
    </div>
  );
}
