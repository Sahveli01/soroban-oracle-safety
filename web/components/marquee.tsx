/**
 * Pure-CSS infinite marquee — guardrail / project keywords scrolling
 * left-to-right. Items are duplicated once so the `translateX(-50%)`
 * loop is seamless. Pauses on hover (see `.marquee-track:hover` in
 * globals.css).
 */
const ITEMS = [
  "DEVIATION",
  "STALENESS",
  "CROSS-SOURCE",
  "LIQUIDITY",
  "THIN SAMPLING",
  "CIRCUIT BREAKER",
  "ED25519",
  "SOROBAN",
  "STELLAR SDEX",
  "AUDITED",
  "OPEN SOURCE",
  "290 TESTS PASSING",
];

export function Marquee() {
  const items = [...ITEMS, ...ITEMS];

  return (
    <div className="marquee-mask relative overflow-hidden border-y border-[var(--color-border)] py-6">
      <div className="marquee-track flex gap-12 whitespace-nowrap">
        {items.map((item, i) => (
          <div
            key={i}
            className="flex items-center gap-12 font-mono text-sm uppercase tracking-widest text-[var(--color-text-muted)]"
          >
            <span>{item}</span>
            <span className="text-[var(--color-accent)]">·</span>
          </div>
        ))}
      </div>
    </div>
  );
}
