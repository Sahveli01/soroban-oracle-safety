"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

interface SinkCard {
  name: string;
  description: string;
  envKey: string;
  setupNote: string;
}

const SINKS: SinkCard[] = [
  {
    name: "Discord",
    description:
      "Incoming webhook to any channel. Alert body wrapped with a bold ⚠️ prefix.",
    envKey: "ORACLE_WATCH_DISCORD_WEBHOOK_URL",
    setupNote: "Server Settings → Integrations → Webhooks",
  },
  {
    name: "Telegram",
    description:
      "Bot sendMessage to a channel or group. Plain-text alert body, no markup.",
    envKey: "ORACLE_WATCH_TELEGRAM_BOT_TOKEN",
    setupNote: "Talk to @BotFather, create bot, copy token + chat ID",
  },
  {
    name: "Slack",
    description:
      "Block Kit message — header plus code-blocked body, fixed amber accent.",
    envKey: "ORACLE_WATCH_SLACK_WEBHOOK_URL",
    setupNote: "Apps → Incoming Webhooks → Add to channel",
  },
  {
    name: "PagerDuty",
    description:
      "Events API v2 incident. Dedup-key is a stable hash of the body — repeats collapse to one incident.",
    envKey: "ORACLE_WATCH_PAGERDUTY_INTEGRATION_KEY",
    setupNote: "Services → New Service → Events API v2 integration",
  },
  {
    name: "Generic",
    description:
      "POST { message, source } JSON to any URL. Optional custom headers. Most flexible.",
    envKey: "ORACLE_WATCH_GENERIC_WEBHOOK_URL",
    setupNote: "Your endpoint receives a standardized JSON payload",
  },
];

export function Operator() {
  return (
    <SectionShell id="operator" eyebrow="Operator">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: [0.19, 1, 0.22, 1] }}
        className="text-4xl font-medium leading-[1.1] tracking-tight sm:text-5xl md:text-6xl"
      >
        Plug in your stack.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-8 max-w-2xl text-lg leading-relaxed text-text-muted"
      >
        oracle-watch dispatches the same alert message to every configured
        sink. Add a webhook URL, deploy. Five sinks shipped, more easily added.
      </motion.p>

      {/* Sink grid */}
      <div className="mt-16 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {SINKS.map((sink, i) => (
          <motion.div
            key={sink.name}
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{
              delay: i * 0.06,
              duration: 0.5,
              ease: [0.19, 1, 0.22, 1],
            }}
            className="group flex flex-col rounded-xl border border-border bg-surface p-6 transition-all hover:border-accent/40"
          >
            {/* Sink name */}
            <h3 className="font-mono text-sm uppercase tracking-wider text-accent">
              {sink.name}
            </h3>

            {/* Description */}
            <p className="mt-4 text-text">{sink.description}</p>

            {/* Env example — appears on hover, subtle */}
            <div className="mt-6 flex-1 space-y-2 opacity-70 transition-opacity group-hover:opacity-100">
              <div className="font-mono text-xs text-text-dim">SETUP</div>
              <div className="font-mono text-xs text-text-muted">
                {sink.setupNote}
              </div>
              <div className="mt-3 rounded border border-border bg-background px-3 py-2 font-mono text-xs text-text-muted">
                {sink.envKey}
              </div>
            </div>

            {/* Footer hint */}
            <div className="mt-6 flex items-center gap-2 font-mono text-xs text-text-dim">
              <span className="inline-block h-px w-6 bg-accent" />
              <span>Setup in 30 seconds</span>
            </div>
          </motion.div>
        ))}
      </div>

      {/* Custom sink note */}
      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.6, duration: 0.6 }}
        className="mt-16 rounded-xl border border-border bg-surface p-8"
      >
        <div className="grid gap-8 md:grid-cols-12 md:items-center">
          <div className="md:col-span-7">
            <h3 className="font-mono text-xs uppercase tracking-wider text-text-muted">
              Want a sink we don&apos;t have?
            </h3>
            <p className="mt-4 text-lg text-text">
              Implement the{" "}
              <span className="font-mono text-accent">WebhookSink</span> trait
              (<span className="font-mono text-text-muted">kind() + send()</span>
              ), register it in{" "}
              <span className="font-mono text-accent">
                AlertConfig::build_sinks()
              </span>
              .
            </p>
            <p className="mt-2 text-sm text-text-muted">
              The pattern is identical for any HTTPS endpoint. The five shipped
              sinks average ~200 lines each, tests included.
            </p>
          </div>
          <div className="md:col-span-5">
            <div className="rounded-lg border border-border bg-background p-4 font-mono text-xs leading-relaxed">
              <div className="text-text-dim">// Custom sink</div>
              <div className="mt-2 text-text-muted">
                <span className="text-accent">impl</span> WebhookSink{" "}
                <span className="text-accent">for</span> MySink {`{`}
              </div>
              <div className="ml-4 text-text-muted">
                <span className="text-accent">fn</span> kind() -&gt; &amp;str{" "}
                {`{`} <span className="text-text">&quot;my-sink&quot;</span>{" "}
                {`}`}
              </div>
              <div className="ml-4 text-text-muted">
                <span className="text-accent">async fn</span> send(msg) {`{`}{" "}
                ... {`}`}
              </div>
              <div className="text-text-muted">{`}`}</div>
            </div>
          </div>
        </div>
      </motion.div>
    </SectionShell>
  );
}
