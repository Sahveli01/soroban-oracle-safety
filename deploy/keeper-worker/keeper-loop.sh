#!/usr/bin/env bash
#
# keeper-loop — always-on wrapper around scripts/keeper.sh.
#
# Runs the real keeper (2 ticks/asset) every LOOP_INTERVAL seconds, forever, so
# the noeracle-adapter ring buffer stays well inside safe-oracle's 900s staleness
# window and Noeracle stays APPROVED on the live demo. This replaces the
# GitHub Actions cron, whose scheduling jitter let the buffer go stale.
#
# The keeper logic itself lives in keeper.sh (the single source of truth — this
# file NEVER duplicates it). We only add one-time key/network setup, then loop.
#
# Secret handling: DEPLOYER_SECRET_KEY arrives as a Fly.io secret (env var). It
# is written straight into the stellar identity file via printf (a shell
# builtin, redirected to the file) and is NEVER echoed. There is no `set -x`.
# This is a TESTNET key; the worker only ever touches testnet.
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${KEEPER_SOURCE:-deployer}"
RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"
PASSPHRASE="${NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}"
# Seconds to sleep between full keeper runs. Must stay well under safe-oracle's
# 900s staleness window. A run takes ~60-90s, so 150s → ~4 min/cycle ≪ 900s.
LOOP_INTERVAL="${LOOP_INTERVAL:-150}"

HERE="$(cd "$(dirname "$0")" && pwd)"
# Locate the single source of truth, keeper.sh. In the container the Dockerfile
# copies it next to this script (/app/keeper.sh); in the repo it lives at
# scripts/keeper.sh. Honor an explicit KEEPER_SCRIPT override first.
KEEPER_SCRIPT="${KEEPER_SCRIPT:-}"
if [ -z "$KEEPER_SCRIPT" ]; then
  if [ -f "$HERE/keeper.sh" ]; then
    KEEPER_SCRIPT="$HERE/keeper.sh"
  elif [ -f "$HERE/../../scripts/keeper.sh" ]; then
    KEEPER_SCRIPT="$HERE/../../scripts/keeper.sh"
  else
    echo "FATAL: cannot find keeper.sh (set KEEPER_SCRIPT)" >&2
    exit 1
  fi
fi
# keeper.sh reads these from the environment.
export NETWORK KEEPER_SOURCE="$SOURCE"

# --- one-time setup (idempotent across container restarts) -------------------
stellar network add "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$PASSPHRASE" \
  --overwrite >/dev/null 2>&1 || true

# Import the keeper secret non-interactively. Stellar CLI 25.1.0's
# `keys add --secret-key` is a TTY prompt (hangs headless), so we write the
# identity file directly. The secret never reaches stdout/stderr; umask 077
# keeps the file owner-only.
if [ -n "${DEPLOYER_SECRET_KEY:-}" ]; then
  umask 077
  mkdir -p "$HOME/.config/stellar/identity"
  printf 'secret_key = "%s"\n' "$DEPLOYER_SECRET_KEY" > "$HOME/.config/stellar/identity/${SOURCE}.toml"
elif stellar keys address "$SOURCE" >/dev/null 2>&1; then
  : # alias already present (e.g. local dev) — reuse it, no secret needed
else
  echo "FATAL: DEPLOYER_SECRET_KEY not set and no '$SOURCE' identity found" >&2
  exit 1
fi
echo "keeper worker up — signing as $(stellar keys address "$SOURCE") on $NETWORK, interval=${LOOP_INTERVAL}s"

# --- continuous feed loop ----------------------------------------------------
cycle=0
while true; do
  cycle=$((cycle + 1))
  echo "===== keeper cycle #$cycle @ $(date -u +%FT%TZ) ====="
  # A failed cycle (RPC hiccup, partial relay) must NOT kill the worker: log it
  # and retry next interval. The `if` guard suspends `set -e` for keeper.sh.
  if bash "$KEEPER_SCRIPT"; then
    echo "cycle #$cycle complete"
  else
    echo "cycle #$cycle failed (exit $?) — retrying after interval" >&2
  fi
  sleep "$LOOP_INTERVAL"
done
