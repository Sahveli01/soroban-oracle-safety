# Noeracle Keeper — always-on Fly.io worker

A 24/7 background worker that keeps the Noeracle oracle **APPROVED** on the live
demo. It wraps the repo's real [`scripts/keeper.sh`](../../scripts/keeper.sh)
(2 ticks/asset) in an infinite loop, relaying Noeracle's signed price
attestations on-chain every `LOOP_INTERVAL` seconds (default **150 s**) so the
`noeracle-adapter` ring buffer never ages past safe-oracle's **900 s** staleness
window.

This replaces the GitHub Actions cron (`.github/workflows/noeracle-keeper.yml`),
whose scheduling jitter occasionally let the buffer go stale.

```
deploy/keeper-worker/
├─ Dockerfile        stellar-cli 25.1.0 + bash/curl/python3/jq + the scripts
├─ fly.toml          Fly app config — worker process, no public port, always-on
├─ keeper-loop.sh    infinite loop: key/network setup → keeper.sh → sleep
└─ README.md         this file
```

## What it does each cycle

For **BTC** and **XLM**, twice (so the buffer holds 2 distinct fresh ticks):

1. Fetch the freshly-signed attestation from Noeracle's public API.
2. Relay it to Noeracle via `update_ed25519_persistent` (refreshes `get_price_pers`).
3. Write it into the adapter ring buffer via `lastprice` — what safe-oracle's
   deviation guardrail reads via `prices(asset, 2)`.

Contracts (testnet):

| role      | contract id                                                |
|-----------|------------------------------------------------------------|
| Noeracle  | `CAYIP67UDVX5UPXGN3XDAWVIEFBAVG6G7LUESEOU3NUQKTWN55W34YBG`  |
| Adapter   | `CBTGC7YL2SV7BAWSJ72WLZGKRCSMXZNTIVXNOAR2V2LCXQRCBOWNUBFX`  |
| Validator | `CBMDP4NLNLI2T5XJPMGT23QUXZKLD67YX6YGVNUKPLEFRCJG3ICUJX4K`  |

## Deploy

> Run everything **from the repo root** (`soroban-oracle-safety/`). The build
> context must be the repo root so the Dockerfile can `COPY scripts/keeper.sh`.

```bash
# 1. Log in to Fly.io
flyctl auth login

# 2. Create the app (skip if it already exists). `app` in fly.toml must be
#    globally unique — pick another name here AND in fly.toml if it's taken.
flyctl apps create noeracle-keeper

# 3. Set the keeper secret. THIS IS A TESTNET KEY. It is stored as a Fly secret
#    and is never written to logs. Paste your own deployer secret (S...):
flyctl secrets set DEPLOYER_SECRET_KEY=S... --app noeracle-keeper

# 4. Deploy (context = ".", the repo root)
flyctl deploy . --config deploy/keeper-worker/fly.toml \
                --dockerfile deploy/keeper-worker/Dockerfile

# 5. Watch it feed
flyctl logs --app noeracle-keeper
```

You should see, every ~150 s: `===== keeper cycle #N =====`, then per-asset
`relay tx` / `buffer tx` hashes, then `cycle #N complete`. **No secret ever
appears in the logs.**

## Verify it's working

```bash
ADAPTER=CBTGC7YL2SV7BAWSJ72WLZGKRCSMXZNTIVXNOAR2V2LCXQRCBOWNUBFX
VALIDATOR=CBMDP4NLNLI2T5XJPMGT23QUXZKLD67YX6YGVNUKPLEFRCJG3ICUJX4K

# 2 fresh, distinct-timestamp entries:
stellar contract invoke --id "$ADAPTER" --network testnet --source <acc> --send=no -- \
  prices --asset '{"Other":"BTC"}' --records 2

# APPROVED (approved:true, violation:0):
stellar contract invoke --id "$VALIDATOR" --network testnet --source <acc> --send=no -- \
  validate --oracle "$ADAPTER" --asset '{"Other":"BTC"}'
```

## Configuration (fly.toml `[env]`)

| var             | default   | meaning                                            |
|-----------------|-----------|----------------------------------------------------|
| `LOOP_INTERVAL` | `150`     | seconds between full keeper runs (keep ≪ 900 s)    |
| `NETWORK`       | `testnet` | stellar network name                               |
| `KEEPER_SOURCE` | `deployer`| identity alias the worker signs with               |
| `TICK_GAP`      | `6`       | seconds between the 2 ticks (set on the worker)    |
| `TICKS_PER_RUN` | `2`       | ticks per asset per run                             |

`TICK_GAP` / `TICKS_PER_RUN` are read by `keeper.sh`; add them to `[env]` to
override.

## Always-on guarantee

The worker has no service/port, so Fly never scales it to zero. The loop never
exits (a failed cycle is logged and retried next interval), and Fly restarts the
machine if the process ever dies — so the feed is continuous.

## Local smoke test (no Fly, no secret)

If a `deployer` identity already exists in your local stellar config, the loop
reuses it (no `DEPLOYER_SECRET_KEY` needed):

```bash
LOOP_INTERVAL=20 bash deploy/keeper-worker/keeper-loop.sh   # Ctrl-C to stop
```
