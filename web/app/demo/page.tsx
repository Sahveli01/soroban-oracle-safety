import type { Metadata } from "next";
import Link from "next/link";
import MultiOracle from "@/components/demo/multi-oracle";
import AttackReplay from "@/components/demo/attack-replay";

export const metadata: Metadata = {
  title: "safe-oracle — Live Demo",
  description:
    "Run safe-oracle's guardrails live against real Stellar testnet oracles, and replay real oracle-manipulation attacks against its default thresholds.",
};

export default function DemoPage() {
  return (
    <main className="screen-min overflow-x-hidden">
      <div className="mx-auto w-full max-w-5xl px-5 pb-24 sm:px-8">
        {/* Top bar */}
        <header className="flex items-center justify-between py-6">
          <Link
            href="/"
            className="link-sweep font-mono text-sm text-text-muted hover:text-text"
          >
            ← safe-oracle
          </Link>
          <span className="t-eyebrow">Live demo</span>
        </header>

        {/* Hero */}
        <section className="pt-10 pb-14 sm:pt-16">
          <h1 className="t-h1 max-w-3xl">
            One safety layer.
            <br />
            <span className="text-text-muted">Every oracle, live.</span>
          </h1>
          <p className="mt-6 max-w-xl text-[15px] leading-relaxed text-text-muted">
            These aren&apos;t mockups. Every verdict below is a read-only{" "}
            <span className="font-mono text-text">simulateTransaction</span>{" "}
            against a validator contract on Stellar testnet — the same
            safe-oracle guardrails your protocol would call.
          </p>
        </section>

        {/* Demo 1 */}
        <section className="border-t border-border pt-14">
          <div className="mb-8">
            <p className="t-eyebrow mb-3">Demo 1 · Multi-oracle</p>
            <h2 className="t-h2 max-w-xl">
              Trust the oracle. Verify it anyway.
            </h2>
          </div>
          <MultiOracle />
        </section>

        {/* Demo 2 */}
        <section className="mt-20 border-t border-border pt-14">
          <div className="mb-8">
            <p className="t-eyebrow mb-3">Demo 2 · Attack replay</p>
            <h2 className="t-h2 max-w-xl">
              Replay the heists. Watch the boundary hold.
            </h2>
          </div>
          <AttackReplay />
        </section>

        {/* Honesty line — the long provenance paragraph is gone; transparency
            now lives as short labels inside each demo (feed notes, Band/DIA
            "not deployed", "real attack, real prices"). */}
        <footer className="mt-16 border-t border-border pt-6">
          <p className="font-mono text-[11px] text-text-dim">
            Live Stellar testnet · read-only simulation · no prices fabricated.
          </p>
        </footer>
      </div>
    </main>
  );
}
