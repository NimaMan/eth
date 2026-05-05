# eth_token examples

Rust-only examples for validating and inspecting token-state processing.

## Layout

- `replay/`: Process historical blocks and rebuild token or pool state from `tx_processor` output.
- `validation/`: Compare reconstructed token state against direct chain reads.
- `analysis/`: Human-readable inspections for lifecycle, liquidity, and health.
- `fixtures/`: Known token/pool/block ranges reused by examples.

## Current Examples

### `uniswap_v2_pool_replay_reserves`

Replays a Uniswap V2 pool over a block range, updates the Rust `eth_token` pool state, and compares the final reconstructed reserves with `getReserves()` at the end block.

Run with explicit inputs:

```bash
cargo run -p eth_token --example uniswap_v2_pool_replay_reserves -- \
  --token 0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2 \
  --pool 0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7 \
  --start 23196199 \
  --end 23196202
```

Run a known fixture:

```bash
cargo run -p eth_token --example uniswap_v2_pool_replay_reserves -- \
  --known moo_weth_launch
```

Set `RETH_DATADIR` or pass `--datadir` when the default local Reth path is not available.
