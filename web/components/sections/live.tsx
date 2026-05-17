"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

/**
 * Values pulled from `deployment/testnet.json`. Hashes link to
 * stellar.expert testnet explorer (verifiable on-chain).
 */
const CONTRACTS = [
  {
    name: "LiquidityRegistry",
    addr: "CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND",
    short: "CCDWMKL5…WGND",
  },
  {
    name: "mock-lending",
    addr: "CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV",
    short: "CA6TBUXT…MXZV",
  },
  {
    name: "mock-reflector",
    addr: "CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7",
    short: "CBUPTLPD…PHO7",
  },
];

type TxStatus = "success" | "attack" | "rejected" | "recovery";

const TXS: {
  label: string;
  hash: string;
  ledger?: string;
  error?: string;
  status: TxStatus;
}[] = [
  {
    label: "Successful borrow",
    hash: "ce4812031daa61ecb987c45123fbaba52eb83fe0b27f623dd3fa3fa0ec8a5c45",
    ledger: "2,450,314",
    status: "success",
  },
  {
    label: "Attack: 10× price spike",
    hash: "b99d61340c63748394f27a589ac228bbc6a02aba7d74c5b50b67a416ee6acfb6",
    status: "attack",
  },
  {
    label: "Adversarial REJECTED",
    hash: "a1cfdec1fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653",
    error: "ExcessiveDeviation",
    status: "rejected",
  },
  {
    label: "Recovery",
    hash: "9cae263874ab308ccba3871bc00aeec95dbff0199e2e7187c71c1ecf1bba378f",
    status: "recovery",
  },
];

const STATUS_DOT: Record<TxStatus, string> = {
  success: "bg-accent",
  attack: "bg-danger",
  rejected: "bg-accent",
  recovery: "bg-text-dim",
};

const STATUS_LABEL: Record<TxStatus, string> = {
  success: "SUCCESS",
  attack: "ATTACK",
  rejected: "REJECTED",
  recovery: "RECOVERY",
};

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

function shortHash(hash: string): string {
  return `${hash.slice(0, 8)}…${hash.slice(-6)}`;
}

/** Clean editorial row — hairline divider, generous height, big type. */
function Row({
  href,
  primary,
  secondary,
  badge,
  dot,
  delay,
}: {
  href: string;
  primary: string;
  secondary: string;
  badge?: string;
  dot?: string;
  delay: number;
}) {
  return (
    <motion.a
      initial={{ opacity: 0, y: 10 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-40px" }}
      transition={{ delay, duration: 0.5, ease: EASE }}
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="group flex items-center justify-between gap-6 border-b border-border py-5 transition-colors hover:border-accent/40"
    >
      <div className="min-w-0">
        <div className="flex items-center gap-3">
          {dot && (
            <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${dot}`} />
          )}
          <span className="truncate text-lg font-medium md:text-xl">
            {primary}
          </span>
          {badge && (
            <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.2em] text-text-dim">
              {badge}
            </span>
          )}
        </div>
        <div className="mt-1.5 truncate pl-0 font-mono text-sm text-text-muted">
          {secondary}
        </div>
      </div>
      <span className="shrink-0 font-mono text-sm text-text-dim transition-all group-hover:translate-x-0.5 group-hover:text-accent">
        ↗
      </span>
    </motion.a>
  );
}

export function Live() {
  return (
    <SectionShell id="live" eyebrow="Live on Stellar">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.6, ease: EASE }}
        className="flex items-center gap-2.5 font-mono text-xs uppercase tracking-[0.25em] text-text-muted"
      >
        <span className="relative flex h-2 w-2">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-75" />
          <span className="relative inline-flex h-2 w-2 rounded-full bg-accent" />
        </span>
        Testnet · operational
      </motion.div>

      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.1, duration: 0.7, ease: EASE }}
        className="mt-6 t-h1"
      >
        Proven on-chain.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-6 max-w-2xl text-lg leading-relaxed text-text-muted"
      >
        Three contracts deployed. 17 oracle-watch attestations. The first
        adversarial replay rejected at the protocol layer — every hash public
        and verifiable.
      </motion.p>

      <div className="mt-14 grid gap-x-16 gap-y-12 lg:grid-cols-2">
        <div>
          <h3 className="font-mono text-xs uppercase tracking-[0.25em] text-text-dim">
            Deployed Contracts
          </h3>
          <div className="mt-2 border-t border-border">
            {CONTRACTS.map((c, i) => (
              <Row
                key={c.name}
                href={`https://stellar.expert/explorer/testnet/contract/${c.addr}`}
                primary={c.name}
                secondary={c.short}
                delay={i * 0.06}
              />
            ))}
          </div>
        </div>

        <div>
          <h3 className="font-mono text-xs uppercase tracking-[0.25em] text-text-dim">
            End-to-End Validation
          </h3>
          <div className="mt-2 border-t border-border">
            {TXS.map((tx, i) => (
              <Row
                key={tx.hash}
                href={`https://stellar.expert/explorer/testnet/tx/${tx.hash}`}
                primary={tx.label}
                badge={STATUS_LABEL[tx.status]}
                dot={STATUS_DOT[tx.status]}
                secondary={
                  shortHash(tx.hash) +
                  (tx.ledger ? `  ·  ledger ${tx.ledger}` : "") +
                  (tx.error ? `  ·  ${tx.error}` : "")
                }
                delay={i * 0.06}
              />
            ))}
          </div>
        </div>
      </div>
    </SectionShell>
  );
}
