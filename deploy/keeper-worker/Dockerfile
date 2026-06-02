# Always-on Noeracle keeper worker (testnet feed).
#
# Wraps the repo's real scripts/keeper.sh (2 ticks/asset) in an infinite loop so
# the noeracle-adapter ring buffer never ages past safe-oracle's 900s staleness
# window. Testnet only.
#
# IMPORTANT: the build context MUST be the repo ROOT, because we COPY
# scripts/keeper.sh (the single source of truth). Deploy with:
#   flyctl deploy . --config deploy/keeper-worker/fly.toml \
#                   --dockerfile deploy/keeper-worker/Dockerfile
FROM debian:bookworm-slim

ARG STELLAR_CLI_VERSION=25.1.0
ENV DEBIAN_FRONTEND=noninteractive

# Runtime deps, kept minimal:
#   bash            — the scripts
#   curl            — Noeracle API + CLI download
#   ca-certificates — TLS to testnet RPC and api.noeracle.org
#   python3         — keeper.sh parses the attestation JSON with it (NOT jq)
#   jq              — convenience for debugging
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      bash curl ca-certificates python3 jq \
 && rm -rf /var/lib/apt/lists/*

# Stellar CLI — same verified release tarball the GitHub workflow uses.
RUN curl -sSfL \
      "https://github.com/stellar/stellar-cli/releases/download/v${STELLAR_CLI_VERSION}/stellar-cli-${STELLAR_CLI_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
    | tar xz -C /tmp \
 && mv /tmp/stellar /usr/local/bin/stellar \
 && chmod +x /usr/local/bin/stellar \
 && stellar --version

WORKDIR /app
# Single source of truth: the real keeper, plus the loop wrapper around it.
COPY scripts/keeper.sh                   /app/keeper.sh
COPY deploy/keeper-worker/keeper-loop.sh /app/keeper-loop.sh
RUN chmod +x /app/keeper.sh /app/keeper-loop.sh

CMD ["bash", "/app/keeper-loop.sh"]
