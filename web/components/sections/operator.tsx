"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

interface SinkCard {
  name: string;
  description: string;
  envKey: string;
}

const SINKS: SinkCard[] = [
  {
    name: "Discord",
    description: "Incoming webhook to any channel. Bold, alert-prefixed body.",
    envKey: "ORACLE_WATCH_DISCORD_WEBHOOK_URL",
  },
  {
    name: "Telegram",
    description: "Bot sendMessage to a channel or group. Plain-text body.",
    envKey: "ORACLE_WATCH_TELEGRAM_BOT_TOKEN",
  },
  {
    name: "Slack",
    description: "Block Kit message — header + code-blocked body, amber accent.",
    envKey: "ORACLE_WATCH_SLACK_WEBHOOK_URL",
  },
  {
    name: "PagerDuty",
    description: "Events API v2 incident. Dedup-key hash collapses repeats.",
    envKey: "ORACLE_WATCH_PAGERDUTY_INTEGRATION_KEY",
  },
  {
    name: "Generic",
    description: "POST { message, source } JSON to any URL. Custom headers.",
    envKey: "ORACLE_WATCH_GENERIC_WEBHOOK_URL",
  },
];

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

export function Operator() {
  return (
    <SectionShell id="operator" eyebrow="Operator">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: EASE }}
        className="t-h2"
      >
        Plug in your stack.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.15, duration: 0.7 }}
        className="mt-5 max-w-xl text-text-muted"
      >
        oracle-watch dispatches the same alert to every configured sink. Add a
        webhook URL, deploy. Five shipped — more easily added.
      </motion.p>

      <div className="mt-10 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {SINKS.map((sink, i) => (
          <motion.div
            key={sink.name}
            initial={{ opacity: 0, y: 16 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: i * 0.05, duration: 0.45, ease: EASE }}
            className="surface-card group flex flex-col p-5"
          >
            <h3 className="font-mono text-sm uppercase tracking-wider text-text">
              {sink.name}
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-text-muted">
              {sink.description}
            </p>
            <div className="mt-4 truncate rounded border border-border bg-[var(--color-background)] px-2.5 py-1.5 font-mono text-[11px] text-text-dim">
              {sink.envKey}
            </div>
          </motion.div>
        ))}

        {/* Custom-sink card — same grid cell size, no extra height */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ delay: 0.3, duration: 0.45, ease: EASE }}
          className="surface-card flex flex-col justify-between p-5"
        >
          <div>
            <h3 className="font-mono text-sm uppercase tracking-wider text-text">
              Custom sink
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-text-muted">
              Implement <span className="font-mono text-accent">WebhookSink</span>{" "}
              (<span className="font-mono">kind() + send()</span>), register in{" "}
              <span className="font-mono text-accent">build_sinks()</span>.
            </p>
          </div>
          <div className="mt-4 rounded border border-border bg-[var(--color-background)] px-2.5 py-1.5 font-mono text-[11px] text-text-dim">
            impl WebhookSink for MySink {`{ … }`}
          </div>
        </motion.div>
      </div>
    </SectionShell>
  );
}
