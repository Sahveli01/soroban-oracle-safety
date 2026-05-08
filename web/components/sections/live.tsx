"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

/**
 * All values below are pulled from `deployment/testnet.json` at Phase 7 closure.
 * Hashes link to stellar.expert testnet explorer (verifiable on-chain).
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
        transition={{ duration: 0.7 }}
        className="text-5xl font-medium leading-[1.1] tracking-tight md:text-6xl"
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
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-8 max-w-2xl text-text-muted"
      >
        Three contracts deployed. 17 successful oracle-watch attestations.
        First adversarial replay rejected at the protocol layer. All public.
      </motion.p>

      {/* Contract Addresses */}
      <div className="mt-16">
        <h3 className="font-mono text-xs uppercase tracking-wider text-text-muted">
          Deployed Contracts
        </h3>
        <div className="mt-6 space-y-2">
          {CONTRACTS.map((c, i) => (
            <motion.a
              key={c.name}
              initial={{ opacity: 0, x: -10 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true, margin: "-50px" }}
              transition={{ delay: i * 0.08, duration: 0.5 }}
              href={`https://stellar.expert/explorer/testnet/contract/${c.addr}`}
              target="_blank"
              rel="noopener noreferrer"
              className="group flex items-center justify-between rounded-lg border border-border bg-surface p-4 transition-all hover:border-accent/40"
            >
              <div>
                <div className="font-medium">{c.name}</div>
                <div className="mt-1 font-mono text-xs text-text-muted">
                  {c.short}
                </div>
              </div>
              <span className="text-text-dim transition-colors group-hover:text-accent">
                Explorer →
              </span>
            </motion.a>
          ))}
        </div>
      </div>

      {/* Validation Tx Hashes */}
      <div className="mt-16">
        <h3 className="font-mono text-xs uppercase tracking-wider text-text-muted">
          End-to-End Validation
        </h3>
        <div className="mt-6 space-y-2">
          {TXS.map((tx, i) => (
            <motion.a
              key={tx.hash}
              initial={{ opacity: 0, x: -10 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true, margin: "-50px" }}
              transition={{ delay: i * 0.08, duration: 0.5 }}
              href={`https://stellar.expert/explorer/testnet/tx/${tx.hash}`}
              target="_blank"
              rel="noopener noreferrer"
              className="group flex items-center justify-between rounded-lg border border-border bg-surface p-4 transition-all hover:border-accent/40"
            >
              <div className="flex-1">
                <div className="flex items-center gap-3">
                  <span
                    className={`font-mono text-xs ${STATUS_COLOR[tx.status]}`}
                  >
                    {STATUS_LABEL[tx.status]}
                  </span>
                  <span className="font-medium">{tx.label}</span>
                </div>
                <div className="mt-1 font-mono text-xs text-text-muted">
                  {shortHash(tx.hash)}
                  {tx.ledger && (
                    <span className="ml-3">ledger {tx.ledger}</span>
                  )}
                  {tx.error && (
                    <span className="ml-3 text-danger">{tx.error}</span>
                  )}
                </div>
              </div>
              <span className="text-text-dim transition-colors group-hover:text-accent">
                Explorer →
              </span>
            </motion.a>
          ))}
        </div>
      </div>
    </SectionShell>
  );
}
