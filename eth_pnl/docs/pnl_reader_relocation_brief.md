# Brief for Leibniz — move the PnL + token-state READ layer into `eth_pnl`

## Objective
Move the **PnL/address + token-state read layer** out of `eth_risk_atlas` into
**`eth_pnl`** (`blockchains/eth/eth_pnl`), which already owns PnL/token-state
persistence. After this, **`eth_risk_atlas` focuses on risk analytics only**. This is an
**extraction/MOVE, not a rewrite** — preserve behavior and the `/api/v1/eth/addresses`
payload shapes exactly.

## Current state — the monolith breakdown ALREADY landed (uncommitted)
`eth_risk_atlas/src/db/reader.rs` was just split into `eth_risk_atlas/src/db/reader/` (an
**untracked** dir — commit it before/with the move). Module map:

| Module | Contents | Destination |
| --- | --- | --- |
| `reader/trader.rs` | `eth_traders` (list), `eth_trader_profile`, `eth_trader_trade` | **MOVE → eth_pnl** |
| `reader/trader_sql.rs` | all `eth_trader_*` SQL builders (`ranked`/`filtered`/`positions`/`breakdown`/`movements`/`rows`) | **MOVE → eth_pnl** |
| `reader/trader_rows.rs` | JSON mappers: `eth_trader_row`, `eth_address_aggregate_json`, **`eth_address_type_and_flags`** (admissibility gate), `eth_trader_pool_position_row`, `eth_trader_movement_row`, breakdown rows; the **`INFRA_ADDRESSES`** set + `SCAM_DOMINANCE_THRESHOLD` | **MOVE → eth_pnl** |
| `reader/run_metadata.rs` | `latest_token_pnl_run` (+ `token_pnl_run_json`) → MOVE; `latest_page_view`/`runs`/`page_view` → STAY | **SPLIT** |
| `reader/scammer.rs` | `scammer_address_distribution` + `scammer_address_*_sql` (risk analytics) | **STAY in risk_atlas** |
| token-state reads | any `token_latest`/`pool_latest` query fns (inventory across the crate) | **MOVE → eth_pnl** (it already has `state/` for writes) |

## STEP 0 — preserve the uncommitted work (already carried, keep it)
The split **preserved** the uncommitted PnL admissibility/taxonomy work — verify it is intact
in `reader/trader_rows.rs`: `tier_one_admissible`/`inadmissible_reasons`, the
`non_eth_denom_units` + `known_infra_address` flags, `INFRA_ADDRESSES`, and the `address_agg`
SUM columns (realized/unrealized/total PnL, gas, win/loss, `all_denom_eth`). Also in flight:
the `terminal_zero → closed_zero_valuation` rename and D2 in `eth_token/src/pnl/accounting/
semantic.rs`. **Commit the current working tree (incl. the untracked `reader/` dir) first**, so
the move starts from a known, buildable state and nothing is lost. Work from working-tree
content, not `HEAD`.

## Shared helper (handle the boundary)
`latest_token_pnl_run` and the PnL `address_agg` aggregates are used by BOTH the trader reader
(moving) AND `scammer.rs` (staying). Move them to `eth_pnl` and have `eth_risk_atlas`
**depend on `eth_pnl`** for run selection / PnL aggregates its analytics need. Do not
duplicate the SQL.

## Suggested layout in `eth_pnl`
`eth_pnl/src/reader/{address_pnl.rs (trader.rs), sql.rs (trader_sql.rs), rows.rs
(trader_rows.rs), run.rs (latest_token_pnl_run)}` + `eth_pnl/src/state/reader.rs`
(token-state reads). Expose via the crate's public API as a reader struct (keep
`EthTraderListParams` and all response JSON keys identical). Add a folder `README.md` + a
top-of-file algorithmic-description comment per the repo CLAUDE.md.

## Re-point callers
- `eth_chain_server/src/http/routes/eth_traders.rs` calls
  `state.risk_atlas.eth_trader_profile/eth_traders/eth_trader_trade` → re-point to the new
  `eth_pnl` reader. Update `ServerState` to hold an `eth_pnl` reader (connect via
  `databases.token_pnl.url`). Update `Cargo.toml` deps. risk_atlas scammer-analytics routes
  keep pointing at risk_atlas.

## Constraints
- **No DB change**: `token_pnl`/`token_state` schemas + config keys unchanged; no migration.
- **Behavior-preserving**: `/api/v1/eth/addresses`, `/:address`, `/:address/trades/:poolId`
  payloads byte-identical (same keys incl. `aggregatePnl`, `addressType`, `validityFlags`,
  `admissibility`). Move, do not redesign. Carry the `closed_zero_valuation` rename through.

## Verify
- `cargo build -p eth_pnl -p eth_risk_atlas -p eth_chain_server` + tests clean.
- Release-build `eth_chain_server`; after restart, `GET /api/v1/eth/addresses/<addr>` returns
  the same JSON, and risk-atlas scammer-analytics endpoints still work.

## Environment caveat
If you run under the bwrap/codex sandbox that blocked Poincare (couldn't `pwd`, no edits),
you cannot edit/build here — confirm filesystem + build access first; if blocked, report back.
