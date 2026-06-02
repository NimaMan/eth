# eth_token examples

Rust-only examples for validating and inspecting token-state processing.

## Layout

- `replay/`: Process historical blocks and rebuild token or pool state from `tx_processor` output.
- `validation/`: Compare reconstructed token state against direct chain reads.
- `network/`: Build token-network and second-order flow-context inspection outputs.
- `custody/`: Inspect holder-custody risks and pool-state flags.
- `analysis/`: Human-readable inspections for lifecycle, liquidity, and health.
- `fixtures/`: Known token/pool/block ranges reused by examples.

## Current Examples

### `pool_custody_flags`

Builds two in-memory pools and prints the explicit pool-state flag projection:
one realized holder-balance drain and one USDT-style latent custody authority.
This demonstrates why custody flags must stay separate from routeability and
reserve liquidity-removal flags.

```bash
cargo run -p eth_token --example pool_custody_flags
```

### `custody_session_balance_drain`

Replays the real Session holder-balance drain case against local Reth data,
showing the event-less internal `transferFrom` and the resulting custody/pool
labels.

```bash
cargo run -p eth_token --example custody_session_balance_drain
```

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

### `token_network_flow_context`

Uses the RethIndex address-block participation index to find candidate token
blocks, loads full processed blocks through `ProcessedBlockProvider`, builds the
token network, then expands the selected token-network seed addresses into the
second-order fund-flow context layer.

```bash
cargo run -p eth_token --example token_network_flow_context -- \
  --token 0x133a79c66bc8789cf4d081159654aa378004541c \
  --start 23000000 \
  --end 23100000 \
  --max-token-blocks 256 \
  --max-seeds 16 \
  --max-blocks-per-address 64
```

Requires `reth_index/address_to_blocks` to be populated for the token and seed
addresses. By default it reads `RETH_INDEX_DIR` or `<RETH_DATADIR>/reth_index`.

### `uniswap_v2_lp_tracker_parity`

Replays a Uniswap V2 pool's LP-token `Transfer` and `Approval` events from a pool-creation range, then compares tracked LP balances, total supply, and allowances against chain `balanceOf`, `totalSupply`, and `allowance` reads at the end block.

Run a known fixture:

```bash
cargo run -p eth_token --example uniswap_v2_lp_tracker_parity -- \
  --known moo_weth_launch
```

For explicit inputs, use a range that starts from pool creation or an empty LP pre-state:

```bash
cargo run -p eth_token --example uniswap_v2_lp_tracker_parity -- \
  --token 0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2 \
  --pool 0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7 \
  --start 23196199 \
  --end 23196202
```
