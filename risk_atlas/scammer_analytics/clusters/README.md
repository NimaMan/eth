# Clusters

## Objective

**Operations** — sets of linked scammers that cut across individual cases, joined
by shared infrastructure. Clustering is how we turn isolated cases into a picture
of one operation (pillar 1: track & flag the malactors), so a packet can name the
whole operation, not just one wallet. See `../DESIGN.md` §2 (entity model) and
§5.3 (cluster attribution).

## Contents (mostly planned)

Per-case `address_clusters.json` already exists under each case's
`artifacts/tx_fund_flow/` (cluster_id, cluster_type, confidence, hub_address,
members, evidence). The cross-case cluster artifacts live here — intended one
folder per cluster, `clusters/<cluster_id>/`, named by the most stable
high-signal identifier (creator wallet, forwarder contract, repeated remover, or
sink family), containing the member scammers, the linking evidence, and a
confidence tier.

Link signals, ranked (`../DESIGN.md` §5.3):

1. Shared cash-out exchange **deposit address** — STRONG.
2. Shared **funder wallet** — STRONG.
3. Repeated **creator / identical token bytecode** — STRONG.
4. Shared **forwarder contract / final sink** — MEDIUM.
5. Tight timing / same launch venue — WEAK (corroborating only).

Never assert a cluster on WEAK-only evidence.

## How it's produced / how it connects

Produced by a cross-case clustering pass (new tool, `../DESIGN.md` §10 step 3)
that joins, across cases, the `address_clusters.json`,
`suspect_cashout_trace.json` nodes/funding, creator addresses, and forwarder
sinks into a link graph. Members are scammers (`../scammers/`); combined
per-exchange exposure draws on `../exchanges/`; the cluster export is a
`../packets/` bundle. Surfaced as `clusters[]` on the read-model endpoint
(`eth_chain_server/src/read_models/analytics/scammer.rs`, §8).

See `../DESIGN.md` and the parent `../README.md`.
