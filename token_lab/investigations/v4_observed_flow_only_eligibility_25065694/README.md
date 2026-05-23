# V4 Observed-Flow Eligibility Classification

Status: confirmed source-data gap. This remains in the active investigation
queue only until the source observation range is rebuilt with runtime
`can_buy/can_sell` and the 15k baseline is rerun.

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

## Required Fix

Rebuild this source observation range from token-server output that preserves
`runtime_state.can_buy` and `runtime_state.can_sell`, then rerun the 15k
historical baseline. These entries should become skipped eligibility decisions,
not failed buy positions.

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
