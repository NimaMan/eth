# V4 Observed-Flow Eligibility Classification

Status: fixed source/simulator parity gap. This remains a canonical regression
case for the rule that V4 source rows must carry runtime trading flags before
Alpha may treat them as buyable.

## Scope

Historical run:
`hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

Range: `25,065,694..25,080,693`

Source replay: `snipe-all-v1-chain-sim-live-v4`

This investigation covers the six Uniswap V4 buy failures in the current 15k
historical baseline.

## Finding

All six failed V4 entries were selected from observations whose top-level
`can_buy=true` and `can_sell=true`, but the stored replay rows only preserve
`runtime_state.last_sync_block` / `runtime_state.last_update_block`. They do not
preserve `runtime_state.can_buy` or `runtime_state.can_sell`.

Same-route probes using the deployed V4 Universal Router failed for every case
at both `block - 1` and the decision block, and at all tested input sizes:

- `0.01 ETH`
- `0.001 ETH`
- `0.0001 ETH`

That makes these source-eligibility pollution under the current replay data,
not a size sensitivity or post-block timing issue.

## Resolution

The current code and DB state close this leakage class:

- Risk Atlas observation rows persist `effective_can_buy` and
  `effective_can_sell`; current V4 rows have both fields populated.
- Current live/backtest strategy observation payloads include
  `pool.runtime_state.can_buy` and `pool.runtime_state.can_sell`.
- `PoolWire::to_pool_snapshot()` refuses to trust old top-level V4
  `can_buy/can_sell` when runtime flags are missing; missing runtime flags become
  `can_buy=false` and `can_sell=false`.
- Local DB check on 2026-05-24 found `1,316,533` V4 Risk Atlas rows with no
  null effective flags, `6,306` V4 strategy observation rows with no missing
  runtime flags, and zero V4 buy-failed execution reports.

The exact historical 15k replay can still be rerun for evidence hygiene, but the
active P0 blocker is resolved: stale V4 observed-flow rows no longer become
eligible buy decisions under current Alpha conversion.

## Failed V4 Entries

| Token | Block | Pool id | Probe result |
| --- | ---: | --- | --- |
| `0xA27a75...92cD5D` | 25,074,555 | `0x3480...135e` | fails at `block-1` and block for all tested sizes |
| `0x1030ad...E0d37a` | 25,074,863 | `0x07f6...d1f9` | fails at `block-1` and block for all tested sizes |
| `0xB52DB3...1F3281` | 25,075,194 | `0x8654...eb77` | fails at `block-1` and block for all tested sizes |
| `0x4a80CD...88e498` | 25,075,942 | `0xfc69...88b8` | fails at `block-1` and block for all tested sizes |
| `0x8ae93F...A0E485` | 25,076,752 | `0x45d5...3722` | fails at `block-1` and block for all tested sizes |
| `0x1699A6...2c2d17` | 25,077,133 | `0x4968...8b17` | fails at `block-1` and block for all tested sizes |

All six revert through Universal Router `0x66a9893cC07D91D95644AEDD05D03f95e1dBA8Af`
with selector `0x91dd7346` at depth 2.

## Follow-Up Check

When we need an exact-window artifact, rerun the historical baseline over
`25,065,694..25,080,693` from current source observations and confirm these
entries are skipped eligibility decisions, not failed buy positions.

## Regression Hooks

- Strategy Lab prints a `Buy Failed Entries` table with protocol, denom,
  observed `can_buy/can_sell`, and the simplified error.
- The probe command shape is:

```bash
cargo run -q -p tx_processor --example probe_uniswap_v4_pool -- \
  --reth-datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --token <TOKEN> \
  --pool-manager 0x000000000004444c5dc75cb358380d2e3de08a90 \
  --pool-id <POOL_ID> \
  --currency0 <TOKEN> \
  --currency1 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2 \
  --denom 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2 \
  --fee 100 \
  --tick-spacing 1 \
  --block <DECISION_BLOCK> \
  --amount-wei 10000000000000000,1000000000000000,100000000000000
```
