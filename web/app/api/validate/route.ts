import { NextRequest, NextResponse } from "next/server";
import {
  Contract,
  TransactionBuilder,
  Address,
  nativeToScVal,
  scValToNative,
  xdr,
  rpc,
  BASE_FEE,
} from "@stellar/stellar-sdk";
import { TESTNET, ORACLES, ASSETS, type OracleId } from "@/lib/demo-data";

// Read-only: builds a `validate(oracle, asset)` call and simulates it against
// the deployed validator. No signature, no fee, no state change — the result
// is whatever safe-oracle's guardrails decide on the live testnet data.

export const runtime = "nodejs";

// Tiny in-memory rate limit (per warm lambda): 30 requests / 10s / IP.
const HITS = new Map<string, number[]>();
function rateLimited(ip: string): boolean {
  const now = Date.now();
  const win = (HITS.get(ip) ?? []).filter((t) => now - t < 10_000);
  win.push(now);
  HITS.set(ip, win);
  return win.length > 30;
}

export async function POST(req: NextRequest) {
  const ip = req.headers.get("x-forwarded-for")?.split(",")[0] ?? "local";
  if (rateLimited(ip)) {
    return NextResponse.json({ error: "rate_limited" }, { status: 429 });
  }

  let body: { oracle?: string; asset?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "bad_json" }, { status: 400 });
  }

  const oracleMeta = ORACLES.find((o) => o.id === body.oracle);
  const asset = body.asset?.toUpperCase();
  if (!oracleMeta || !asset || !ASSETS.includes(asset as never)) {
    return NextResponse.json({ error: "bad_params" }, { status: 400 });
  }
  if (!oracleMeta.address) {
    return NextResponse.json(
      { oracle: oracleMeta.id, asset, unavailable: true },
      { status: 200 },
    );
  }

  try {
    const server = new rpc.Server(TESTNET.rpcUrl);
    const account = await server.getAccount(TESTNET.simSource);
    const validator = new Contract(TESTNET.validator);

    // Asset::Other(Symbol) → contracttype enum encoding: vec[variant, payload].
    const assetScVal = xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol("Other"),
      nativeToScVal(asset, { type: "symbol" }),
    ]);
    const oracleScVal = new Address(oracleMeta.address).toScVal();

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: TESTNET.passphrase,
    })
      .addOperation(validator.call("validate", oracleScVal, assetScVal))
      .setTimeout(30)
      .build();

    const sim = await server.simulateTransaction(tx);
    if (rpc.Api.isSimulationError(sim) || !sim.result) {
      const message = rpc.Api.isSimulationError(sim) ? sim.error : "no_result";
      return NextResponse.json(
        { oracle: oracleMeta.id, asset, error: "sim_failed", detail: message },
        { status: 502 },
      );
    }

    const r = scValToNative(sim.result.retval) as {
      approved: boolean;
      price: bigint;
      timestamp: bigint;
      violation: bigint;
    };

    return NextResponse.json({
      oracle: oracleMeta.id as OracleId,
      asset,
      approved: r.approved,
      price: r.price.toString(),
      timestamp: Number(r.timestamp),
      violation: Number(r.violation),
    });
  } catch (e) {
    return NextResponse.json(
      { oracle: oracleMeta.id, asset, error: "rpc_error", detail: String(e) },
      { status: 502 },
    );
  }
}
