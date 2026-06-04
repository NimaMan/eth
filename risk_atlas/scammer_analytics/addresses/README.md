# Addresses

## Objective

**First-degree screening of any active participant address** — the upstream "is
this address dirty?" assessment that feeds the scammer/cluster pipeline. For ANY
address, surface its **scam-trade ratio** (fraction of its pool positions that are
scams), # scam pools, roles, mechanisms touched, total flow, and the scam
cases/scammers it appears in. Serves pillar 1 (track & flag the malactors); a
high-scam-ratio address is a scammer candidate. See `../DESIGN.md` §2 (level 0),
§3.5 (the assessment), and §9 (the `address/:address` page).

## Contents (computed today — not stored as files yet)

Per-address profiles are **computed in-memory from the token-PnL run**, not stored
here. Source of truth:
`/home/nima/code/crypto/blockchains/eth/risk_atlas/src/db/reader.rs::scammer_address_distribution()`
(top-N distribution today; a per-address variant backs the page). This folder is
the home for materialized per-address screening exports if/when we cache them —
intended shape (`address_assessment`):

- `address`, `scam_ratio`, `scam_pool_position_count` / `pool_position_count`,
  `trade_count`, `total_abs_denom_flow`.
- `role_flags[]`, `scam_mechanisms[]`, `pool_labels[]`.
- `cases[]` / `scammers[]` — the first-degree scam cases/operators this address
  appears in (links forward to the scammer/case pages).
- `{value, confidence, provenance}` per stat (§6): `tool_derived` (PnL-run-based);
  case links are `on_chain_fact`.

## How it connects

The token-PnL run (`run_id`, block range — provenance per §6) →
`scammer_address_distribution()` → the home/landscape "scam-heavy addresses" table
(top-N) and the per-address screening page (`address/:address`). A high-ratio
address with operator roles promotes to a **scammer** (`../scammers/`, §2 level 2).

See `../DESIGN.md` and the parent `../README.md` (Follow the Money objective).
