#!/usr/bin/env bash
#
# noeracle-keeper — relays Noeracle's signed price attestations on-chain.
#
# For each tracked asset, per cycle:
#   1. Fetch the freshly-signed attestation from Noeracle's public API.
#   2. Relay it to the Noeracle contract via `update_ed25519_persistent`
#      (refreshes the otherwise-frozen on-chain `get_price_pers`).
#   3. Call the adapter's `lastprice` as a write so the fresh price lands in
#      the adapter's on-chain ring buffer — which is what `safe-oracle`'s
#      deviation guardrail reads via `prices(asset, 2)`.
#
# The price data is cryptographically Noeracle's (the publisher's ed25519
# signature is verified on-chain); the keeper is transport only. It never
# invents or mutates a price.
#
# Constraints honored:
#   - Noeracle enforces a 60s freshness window → fetch + relay are back-to-back.
#   - round_id is monotonic on-chain → stale rounds are silent no-ops.
#
# Env (all have sensible testnet defaults):
#   NETWORK         stellar network name              (testnet)
#   KEEPER_SOURCE   stellar key alias to sign with    (deployer)
#   NOERACLE        Noeracle contract id
#   ADAPTER         noeracle-adapter contract id
#   NOERACLE_API    attestation API url
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${KEEPER_SOURCE:-deployer}"
NOERACLE="${NOERACLE:-CAYIP67UDVX5UPXGN3XDAWVIEFBAVG6G7LUESEOU3NUQKTWN55W34YBG}"
ADAPTER="${ADAPTER:-CBTGC7YL2SV7BAWSJ72WLZGKRCSMXZNTIVXNOAR2V2LCXQRCBOWNUBFX}"
API="${NOERACLE_API:-https://api.noeracle.org/v1/latest}"
# Pick a Python that actually runs (Windows ships a non-functional `python3`
# App-Store stub; CI ubuntu ships only `python3`). Test each candidate.
PYTHON=""
for _c in python3 python; do
  if command -v "$_c" >/dev/null 2>&1 && "$_c" -c 'import sys, json' >/dev/null 2>&1; then
    PYTHON="$_c"
    break
  fi
done
[ -n "$PYTHON" ] || { echo "no working python found" >&2; exit 1; }

# Asset symbol → Noeracle UTF-8 pair tag (BytesN<8>, hex). Verified live.
tag_for() {
  case "$1" in
    BTC)  echo "4254435553440000" ;; # BTCUSD\0\0
    XLM)  echo "584c4d5553440000" ;; # XLMUSD\0\0
    *) echo "unknown asset: $1" >&2; return 1 ;;
  esac
}

# Extract one attestation field set for PAIR (e.g. BTC/USD) from the API json.
# Emits: price timestamp round_id publisher signature
fetch_fields() {
  local pair="$1"
  # `tr -d '\r'` guards against Windows Python emitting CRLF (a CR would be an
  # illegal control char inside the JSON sig/pubkey arrays).
  curl -s --max-time 15 "$API" | "$PYTHON" -c "
import sys, json
d = json.load(sys.stdin)['assets']['$pair']
print(d['price'], d['timestamp'], d['round_id'], d['publisher'], d['signature'])
" | tr -d '\r'
}

# Pull the tx hash out of stellar's 'Signing transaction: <hash>' line.
tx_hash() { grep -oE '[0-9a-f]{64}' | head -1 || true; }

relay_one() {
  local sym="$1" pair="$2" tag price ts rid pub sig
  tag="$(tag_for "$sym")"
  read -r price ts rid pub sig < <(fetch_fields "$pair")
  echo "[$sym] attestation: price=$price ts=$ts round=$rid"

  # 1) Relay the signed attestation to Noeracle (refresh get_price_pers).
  local rtx
  rtx="$(stellar contract invoke --id "$NOERACLE" --network "$NETWORK" --source "$SOURCE" --send=yes -- \
    update_ed25519_persistent \
    --asset "$tag" --price "$price" --timestamp "$ts" --round_id "$rid" \
    --pubkeys "[\"$pub\"]" --sigs "[\"$sig\"]" 2>&1 | tx_hash)"
  echo "[$sym] relay tx:   ${rtx:-<none>}"

  # 2) Push the fresh on-chain price into the adapter ring buffer (write).
  local ptx
  ptx="$(stellar contract invoke --id "$ADAPTER" --network "$NETWORK" --source "$SOURCE" --send=yes -- \
    lastprice --asset "{\"Other\":\"$sym\"}" 2>&1 | tx_hash)"
  echo "[$sym] buffer tx:  ${ptx:-<none>} (ts=$ts)"
}

echo "noeracle-keeper run — network=$NETWORK noeracle=$NOERACLE adapter=$ADAPTER"
relay_one BTC BTC/USD
relay_one XLM XLM/USD
echo "noeracle-keeper run complete"
