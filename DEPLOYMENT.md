# DEPLOYMENT.md

Operator and integrator guide for `safe-oracle`.

---

## Quick Reference — Testnet Contracts

```
Network:      Stellar Testnet
Passphrase:   "Test SDF Network ; September 2015"
Horizon:      https://horizon-testnet.stellar.org
Soroban RPC:  https://soroban-testnet.stellar.org

LiquidityRegistry:  CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND
mock-lending:       CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV
mock-reflector:     CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7

XLM SAC (testnet):  CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

For full audit trail (deploy timestamps, init tx hashes, bug fixes, e2e validation), see [`deployment/testnet.json`](deployment/testnet.json).

---

## Integration Guide — Using safe-oracle in Your Contract

### Step 1: Add Dependency

In your `Cargo.toml`:

```toml
[dependencies]
safe-oracle = { path = "../safe-oracle" }  # path-dep until published
soroban-sdk = "25.3"
```

### Step 2: Call `lastprice` in Your Contract

```rust
use safe_oracle::{lastprice, Asset, SafeOracleConfig};
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct MyLending;

#[contractimpl]
impl MyLending {
    pub fn safe_borrow(
        env: Env,
        caller: Address,
        asset: Asset,
        amount: i128,
        reflector: Address,
        registry: Address,
        config: SafeOracleConfig,
    ) -> Result<(), MyError> {
        caller.require_auth_for_args((asset.clone(), amount).into_val(&env));

        let price_data = lastprice(&env, &asset, &reflector, &registry, &config)
            .into_result()
            .map_err(MyError::from)?;

        // Now `price_data.price` has passed all 5 guardrails. Use it.
        let collateral_value = amount * price_data.price;
        // ... your business logic

        Ok(())
    }
}
```

### Step 3: Configure Thresholds

`SafeOracleConfig::default()` is calibrated for mainnet. Defaults:

```rust
SafeOracleConfig {
    max_deviation_bps: 2000,                // 20%
    max_staleness_seconds: 300,              // 5 minutes
    previous_max_staleness_seconds: 900,     // 15 minutes (3× current)
    max_cross_source_bps: 500,               // 5% (when secondary set)
    max_snapshot_age_seconds: 300,           // 5 minutes
    min_liquidity_usd: 100_000_000_000,      // $10,000 (7-decimal stroop)
    min_trade_count_1h: 5,
    secondary_oracle: None,                  // single-source by default
    circuit_breaker_enabled: false,          // opt-in (Phase 5)
    circuit_breaker_halt_ledgers: 720,       // ~1 hour at 5s/ledger
}
```

For custom thresholds, validate before storing:

```rust
let config = SafeOracleConfig { /* ... */ };
config.validate().expect("config must be valid by construction");
env.storage().instance().set(&DataKey::Config, &config);
```

`validate()` rejects silent-disable configurations (e.g., `min_liquidity_usd == 0`, `max_deviation_bps == 0`).

---

## Operator Guide — Running oracle-watch

`oracle-watch` is the off-chain companion service that:

- Polls SDEX trade flow via Horizon `/trades`
- Aggregates 30-min volume + 1-hour unique trader count
- Signs and submits liquidity snapshots to `LiquidityRegistry`
- Detects anomalies (volume drop, trader concentration, price gap) and dispatches alerts via `WebhookSink` (Discord/Telegram out of the box)

### Prerequisites

- Rust 1.85+
- Stellar testnet keypair funded via Friendbot
- Whitelisted as attester in `LiquidityRegistry` (admin must call `add_attester`)

### Configuration

Create `.env.testnet` (see `.gitignore` — never commit secrets):

```bash
NETWORK=testnet
ORACLE_WATCH_HORIZON_URL=https://horizon-testnet.stellar.org
ORACLE_WATCH_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
ORACLE_WATCH_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

ORACLE_WATCH_REGISTRY_CONTRACT_ID=CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND
ORACLE_WATCH_SIGNING_SECRET_KEY=<hex 32-byte ed25519 secret>

# Watched assets: CODE:ISSUER:SAC_CONTRACT_ID (3-part, comma-separated for multiple)
ORACLE_WATCH_WATCHED_ASSETS=XLM:native:CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC

# Counter asset for SDEX pair queries (USDC/native = XLM/USDC pair)
ORACLE_WATCH_COUNTER_ASSET_CODE=USDC
ORACLE_WATCH_COUNTER_ASSET_ISSUER=GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5

ORACLE_WATCH_POLL_INTERVAL_LEDGERS=5
ORACLE_WATCH_MAX_SNAPSHOT_AGE_SECONDS=300
ORACLE_WATCH_USDC_PRICE_USD=1.0

# Optional: alerts (set both for Telegram, just URL for Discord)
# ORACLE_WATCH_DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
# ORACLE_WATCH_TELEGRAM_BOT_TOKEN=...
# ORACLE_WATCH_TELEGRAM_CHAT_ID=...
```

### Run

```bash
# Build
cargo build --release -p oracle-watch

# Load env and run (PowerShell-equivalent on Windows)
export $(grep -v '^#' .env.testnet | xargs)
./target/release/oracle-watch
```

Expected output:

```
oracle-watch starting...
oracle-watch config loaded: 1 watched assets, poll interval 25s, max snapshot age 300s
oracle-watch attester pubkey: 3b90746c607f0ce9105be7b87fd5438def14c8715408cc5eae91609991e78c41
oracle-watch registry: horizon=... rpc=... contract=... network="..."
oracle-watch alert sinks configured: 0
oracle-watch entering poll loop
oracle-watch tx <HASH>: ledger <NUM>
oracle-watch iteration: 1 assets, 0 anomalies, 1 writes, 0 write-failures, 0 sign-failures
...
```

Graceful shutdown via Ctrl+C — current iteration completes before the loop exits.

### Discord Webhook Setup

1. Discord server: Settings → Integrations → Webhooks → New Webhook
2. Copy webhook URL
3. Set `ORACLE_WATCH_DISCORD_WEBHOOK_URL=<URL>` in `.env.testnet`

### Telegram Bot Setup

1. Message [@BotFather](https://t.me/BotFather), `/newbot`, follow prompts
2. Copy bot token
3. Add bot to your group, send any message in the group
4. Get chat_id: `https://api.telegram.org/bot<TOKEN>/getUpdates` (look for `chat.id`)
5. Set `ORACLE_WATCH_TELEGRAM_BOT_TOKEN=<TOKEN>` and `ORACLE_WATCH_TELEGRAM_CHAT_ID=<ID>`

### Adding a New Sink

`WebhookSink` is a trait — implement it for any HTTP service (PagerDuty, Opsgenie, internal webhook):

```rust
use crate::monitor::{WebhookSink, Anomaly};
use async_trait::async_trait;

pub struct MySink { url: String }

#[async_trait]
impl WebhookSink for MySink {
    async fn dispatch(&self, anomaly: &Anomaly) -> Result<(), String> {
        // POST to self.url with anomaly serialized as JSON
        Ok(())
    }
}
```

See `crates/oracle-watch/src/discord_sink.rs` and `telegram_sink.rs` for reference implementations.

---

## Reproducing Phase 7.9 Adversarial Replay

To independently verify the library actually rejects oracle manipulation on testnet, replicate the Phase 7.9 attack:

### Stage 1 — Spike Price 10×

```bash
stellar contract invoke \
  --id CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7 \
  --source-account <your testnet keypair> \
  --network testnet \
  --send=yes \
  -- set_price \
    --asset '{"Stellar":"CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"}' \
    --price 100000000000000 \
    --timestamp $(date +%s)
```

Verify the spike took effect:

```bash
stellar contract invoke \
  --id CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7 \
  --source-account <your testnet keypair> \
  --network testnet \
  -- lastprice \
    --asset '{"Stellar":"CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"}'
```

Expected: `{"price":"100000000000000","timestamp":<recent>}`.

### Stage 2 — Borrow Attempt (Should Be Rejected)

```bash
stellar contract invoke \
  --id CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV \
  --source-account <your testnet keypair> \
  --network testnet \
  --send=yes \
  -- borrow \
    --caller $(stellar keys address <your testnet keypair>) \
    --asset '{"Stellar":"CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"}' \
    --amount 1000000000
```

Expected: `{"Failed":1}` = `BorrowOutcome::Failed(MockLendingError::ExcessiveDeviation)`.

This is the same flow that produced tx [`a1cfdec1...`](https://stellar.expert/explorer/testnet/tx/a1cfdec1fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653) on Phase 7.9.

**Important — Ok-only enum design:** The transaction protocol-level status is `SUCCESS` because `BorrowOutcome` is an Ok-only enum at the Soroban boundary. Soroban rolls back all storage writes when a contract method returns `Result::Err`, but the auto-halt circuit breaker needs to commit its writes even on guardrail violations. The `Failed(u32)` variant carries the error discriminant without triggering rollback. See `crates/safe-oracle/src/lib.rs` doc-comment on `PriceResult` for the full design rationale.

### Stage 3 — Restore Price

```bash
stellar contract invoke \
  --id CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7 \
  --source-account <your testnet keypair> \
  --network testnet \
  --send=yes \
  -- set_price \
    --asset '{"Stellar":"CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"}' \
    --price 10000000000000 \
    --timestamp $(date +%s)
```

### Stage 4 — Borrow Recovers

The post-recovery borrow succeeds because the testnet mock-lending was deployed with `circuit_breaker_enabled: false`. With mainnet config (`circuit_breaker_enabled: true`), Stage 4 would be rejected with `CircuitBreakerOpen` for `circuit_breaker_halt_ledgers` (~1 hour) until auto-recovery.

---

## Testnet Configuration Note (IMPORTANT)

**The testnet deployment uses relaxed thresholds for demonstration purposes:**

```rust
SafeOracleConfig {
    max_deviation_bps: 9999,                    // ~100% (mainnet: 2000 = 20%)
    max_staleness_seconds: 86400,               // 24h (mainnet: 300 = 5 min)
    previous_max_staleness_seconds: 86400,      // 24h (mainnet: 900 = 15 min)
    max_snapshot_age_seconds: 86400,            // 24h (mainnet: 300 = 5 min)
    min_liquidity_usd: 1,                       // 1 stroop (mainnet: 100_000_000_000 = $10k)
    min_trade_count_1h: 1,                      // 1 trader (mainnet: 5)
    circuit_breaker_enabled: false,             // disabled (mainnet: true)
    // ...
}
```

This relaxation is necessary because:

1. **Stellar testnet has thin SDEX liquidity.** Real testnet XLM/USDC volume is typically <$10k per 30-minute window, so production `min_liquidity_usd` would always trip.
2. **Reflector mock-prices are infrequently updated.** Production-grade staleness thresholds (300s) would reject snapshots most of the time.
3. **Adversarial reproduction requires `circuit_breaker_enabled: false`** so attackers can replay attacks without waiting for the auto-recovery window.

**Mainnet config restores production-grade thresholds.** The Phase 7.9 adversarial replay still validates the deviation guardrail (90000 BPS attack vs 9999 BPS testnet threshold). Mainnet's 2000 BPS threshold would catch the same attack and any abrupt recovery, requiring price to settle gradually.

---

## Mainnet Preparation Checklist (Phase 9)

Before mainnet deployment, the following items must be addressed:

### Cryptography & Key Management
- [ ] HSM/KMS integration for attester signing key (currently env-var)
- [ ] Multi-attester quorum (currently single-attester model)
- [ ] Key rotation procedure documented and tested

### Configuration
- [ ] Per-asset counter parameterization currently hard-defaults to USDC; supports per-deployment override via env
- [ ] Real-time USD price feed integration (currently `usdc_price_usd: 1.0` static placeholder — see `crates/oracle-watch/src/config.rs` doc-comment)
- [ ] Per-asset threshold tuning (mainnet defaults vs per-asset overrides)

### Reflector Integration
- [ ] Confirm Reflector mainnet contract address
- [ ] Verify Reflector decimals stable at 14
- [ ] Cross-source secondary oracle (e.g., separate Reflector feed) — `secondary_oracle` slot is already wired

### Operations
- [ ] 24/7 oracle-watch monitoring
- [ ] PagerDuty/Opsgenie sink (trait pattern in `monitor.rs` allows easy add)
- [ ] Runbook for circuit breaker manual override (`close_circuit_breaker`)
- [ ] Disaster recovery procedure (attester key compromise)

### Audit
- [ ] Final mainnet audit (current AR.H is library-level)
- [ ] Continuous fuzzing
- [ ] Bug bounty program

---

## Project Files

- `deployment/testnet.json` — Live deployment artifact (contract IDs, deploy/init tx, e2e validations)
- `crates/safe-oracle/` — The library
- `crates/liquidity-registry/` — On-chain attestation contract
- `crates/oracle-watch/` — Off-chain monitor service
- `mocks/mock-reflector/` — Test/dev Reflector with `set_price`
- `mocks/mock-lending/` — Reference integrator showing the 8-line pattern
- `crates/test-utils/` — Shared `TestEnv` harness

---

## Support

- **GitHub:** https://github.com/Sahveli01/soroban-oracle-safety
- **Issues:** https://github.com/Sahveli01/soroban-oracle-safety/issues
- **Stellar Discord:** [Stellar Developers](https://discord.gg/stellardev)
