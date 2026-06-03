// Shared constants + real data for the /demo page.
//
// Everything here is verified live on Stellar testnet (Demo 1) or sourced from
// public exploit post-mortems (Demo 2). No fabricated prices.

export const TESTNET = {
  rpcUrl: "https://soroban-testnet.stellar.org",
  passphrase: "Test SDF Network ; September 2015",
  // Funded account used only as the read-only simulation source (no signature,
  // no fee, no state change). Public key is safe to ship.
  simSource: "GCDVCIWJ7EJCDKSV6DLCOZKOJU54EYWZUSPOB4G6OD5GWIUMUANEMZ2S",
  validator: "CBMDP4NLNLI2T5XJPMGT23QUXZKLD67YX6YGVNUKPLEFRCJG3ICUJX4K",
} as const;

export type OracleId = "reflector" | "noeracle" | "band" | "dia";

export interface OracleMeta {
  id: OracleId;
  label: string;
  /** testnet contract the validator points at; null when not deployed */
  address: string | null;
  /** short provenance line shown under the result */
  note: string;
  live: boolean;
}

export const ORACLES: OracleMeta[] = [
  {
    id: "reflector",
    label: "Reflector",
    address: "CCYOZJCOPG34LLQQ7N24YXBM7LL62R7ONMZ3G6WZAAYPB5OYKOMJRN63",
    note: "Native SEP-40 oracle — read directly, no adapter.",
    live: true,
  },
  {
    id: "noeracle",
    label: "Noeracle",
    address: "CBTGC7YL2SV7BAWSJ72WLZGKRCSMXZNTIVXNOAR2V2LCXQRCBOWNUBFX",
    note: "Adapter-wrapped · history synthesized from Noeracle's signed attestations.",
    live: true,
  },
  {
    id: "band",
    label: "Band",
    address: null,
    note: "Not deployed at the published testnet address — shown honestly as unavailable.",
    live: false,
  },
  {
    id: "dia",
    label: "DIA",
    address: null,
    note: "Not deployed at the published testnet address — shown honestly as unavailable.",
    live: false,
  },
];

export const ASSETS = ["BTC", "XLM"] as const;
export type AssetSym = (typeof ASSETS)[number];

/** OracleSafetyViolation discriminant → human label. 0 = approved. */
export const VIOLATIONS: Record<number, { name: string; blurb: string }> = {
  0: { name: "Approved", blurb: "All Layer 1 guardrails passed." },
  1: { name: "Excessive Deviation", blurb: "Price moved more than the allowed band vs. the previous tick." },
  2: { name: "Stale Data", blurb: "No fresh price (or no two-tick history) within the staleness window." },
  3: { name: "Cross-Source Mismatch", blurb: "Primary and secondary oracle disagree beyond tolerance." },
  8: { name: "External Contract Failure", blurb: "The oracle contract trapped or was unreachable." },
  9: { name: "Decimals Mismatch", blurb: "Primary and secondary report different precision." },
  10: { name: "Unexpected Decimals", blurb: "Oracle precision is not the expected 14 decimals." },
};

/** The Layer 1 checks, in safe-oracle's evaluation order, for the status row.
 *  Order matters: a later check showing ✗ means every earlier one passed. */
export const GUARDRAILS = [
  { key: "external", label: "Feed reachable", violations: [8] },
  { key: "decimals", label: "Decimals = 14", violations: [9, 10] },
  { key: "staleness", label: "Freshness", violations: [2] },
  { key: "deviation", label: "Deviation", violations: [1] },
] as const;

export interface ValidateResult {
  approved: boolean;
  price: string; // i128 as decimal string, 14dp
  timestamp: number;
  violation: number;
  oracle: OracleId;
  asset: string;
  error?: string;
}

/** Format a 14-decimal i128 price string as USD. */
export function formatPrice14(raw: string): string {
  try {
    const n = Number(BigInt(raw)) / 1e14;
    if (!isFinite(n) || n === 0) return "—";
    return n.toLocaleString("en-US", {
      style: "currency",
      currency: "USD",
      maximumFractionDigits: n < 10 ? 4 : 2,
    });
  } catch {
    return "—";
  }
}

// ── Demo 2 — real historical oracle-manipulation attacks ──────────────────
// Prices are from public post-mortems (CFTC / Halborn / Chainalysis etc.),
// already in safe-oracle's 14-decimal convention. safe-oracle's DEFAULT
// max_deviation_bps = 2000 (20%); every one of these is rejected by it.

export interface Attack {
  id: string;
  name: string;
  date: string;
  pair: string;
  lossUsd: number;
  chain: string;
  prePrice: number; // human USD, pre-attack VWAP
  manipPrice: number; // human USD, manipulated peak
  multiple: string;
  guardrail: string;
  summary: string;
  sourceNote: string;
}

export const ATTACKS: Attack[] = [
  {
    id: "yieldblox",
    name: "YieldBlox / Blend V2",
    date: "Feb 2026",
    pair: "USTRY/USDC",
    lossUsd: 10_200_000,
    chain: "Stellar",
    prePrice: 1.0,
    manipPrice: 106.0,
    multiple: "100×",
    guardrail: "Excessive Deviation",
    summary:
      "A single SDEX trade redefined the VWAP window, lifting USTRY from ~$1.00 to ~$106 so the attacker could borrow against phantom collateral.",
    sourceNote: "Native Stellar incident — the exact threat safe-oracle is built for.",
  },
  {
    id: "mango",
    name: "Mango Markets",
    date: "Oct 2022",
    pair: "MNGO/USD",
    lossUsd: 114_000_000,
    chain: "Solana (adapted)",
    prePrice: 0.038,
    manipPrice: 0.91,
    multiple: "24×",
    guardrail: "Excessive Deviation",
    summary:
      "~$4M of cross-exchange buying pumped the MNGO oracle ~24× within ~30 minutes, inflating collateral to drain ~$114M.",
    sourceNote: "CFTC press release 8647-23; replayed against Soroban semantics.",
  },
  {
    id: "bonqdao",
    name: "BonqDAO / Tellor",
    date: "Feb 2023",
    pair: "WALBT/USD",
    lossUsd: 120_000_000,
    chain: "Polygon (adapted)",
    prePrice: 0.05,
    manipPrice: 5000.0,
    multiple: "100,000×",
    guardrail: "Excessive Deviation",
    summary:
      "A spoofed Tellor submitValue inflated WALBT from ~$0.05 to a conservative ~$5,000 stand-in, minting BEUR against worthless collateral.",
    sourceNote: "Conservative lower-bound of the real magnitude (~$1B/token).",
  },
];

/** safe-oracle default deviation threshold, in basis points. */
export const MAX_DEVIATION_BPS = 2000;
