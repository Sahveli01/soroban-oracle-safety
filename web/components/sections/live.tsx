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

const STATUS_COLOR: Record<TxStatus, string> = {
  success: "text-accent",
  attack: "text-danger",
  rejected: "text-accent",
  recovery: "text-text-muted",
};

const STATUS_LABEL: Record<TxStatus, string> = {
  success: "✓ SUCCESS",
  attack: "⚠ ATTACK",
  rejected: "✓ REJECTED",
  recovery: "↻ RECOVERY",
};

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

function shortHash(hash: string): string {
  return `${hash.slice(0, 8)}…${hash.slice(-4)}`;
}

export function Live() {
  return (
    <SectionShell id="live" eyebrow="Live on Stellar">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: EASE }}
        className="text-4xl font-medium leading-[1.05] tracking-tight sm:text-5xl md:text-6xl"
      >
        <span className="inline-flex items-center gap-3">
          <span className="relative flex h-3 w-3">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-75" />
            <span className="relative inline-flex h-3 w-3 rounded-full bg-accent" />
          </span>
          Operational on testnet.
        </span>
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.15, duration: 0.7 }}
        className="mt-5 max-w-xl text-text-muted"
      >
        Three contracts deployed. 17 oracle-watch attestations. First
        adversarial replay rejected at the protocol layer. All public.
      </motion.p>

      {/* Two columns — contracts + validation side by side to cut height */}
      <div className="mt-10 grid gap-x-10 gap-y-8 lg:grid-cols-2">
        <div>
          <h3 className="font-mono text-[11px] uppercase tracking-[0.2em] text-text-dim">
            Deployed Contracts
          </h3>
          <div className="mt-4 space-y-2">
            {CONTRACTS.map((c, i) => (
              <motion.a
                key={c.name}
                initial={{ opacity: 0, y: 10 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ delay: i * 0.06, duration: 0.45, ease: EASE }}
                href={`https://stellar.expert/explorer/testnet/contract/${c.addr}`}
                target="_blank"
                rel="noopener noreferrer"
                className="surface-card group flex items-center justify-between px-4 py-3"
              >
                <div>
                  <div className="text-sm font-medium">{c.name}</div>
                  <div className="mt-0.5 font-mono text-xs text-text-muted">
                    {c.short}
                  </div>
                </div>
                <span className="font-mono text-xs text-text-dim transition-colors group-hover:text-accent">
                  ↗
                </span>
              </motion.a>
            ))}
          </div>
        </div>

        <div>
          <h3 className="font-mono text-[11px] uppercase tracking-[0.2em] text-text-dim">
            End-to-End Validation
          </h3>
          <div className="mt-4 space-y-2">
            {TXS.map((tx, i) => (
              <motion.a
                key={tx.hash}
                initial={{ opacity: 0, y: 10 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ delay: i * 0.06, duration: 0.45, ease: EASE }}
                href={`https://stellar.expert/explorer/testnet/tx/${tx.hash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="surface-card group flex items-center justify-between px-4 py-3"
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span
                      className={`font-mono text-[11px] ${STATUS_COLOR[tx.status]}`}
                    >
                      {STATUS_LABEL[tx.status]}
                    </span>
                    <span className="truncate text-sm font-medium">
                      {tx.label}
                    </span>
                  </div>
                  <div className="mt-0.5 font-mono text-xs text-text-muted">
                    {shortHash(tx.hash)}
                    {tx.ledger && (
                      <span className="ml-2 text-text-dim">
                        L{tx.ledger}
                      </span>
                    )}
                    {tx.error && (
                      <span className="ml-2 text-danger">{tx.error}</span>
                    )}
                  </div>
                </div>
                <span className="font-mono text-xs text-text-dim transition-colors group-hover:text-accent">
                  ↗
                </span>
              </motion.a>
            ))}
          </div>
        </div>
      </div>
    </SectionShell>
  );
}
