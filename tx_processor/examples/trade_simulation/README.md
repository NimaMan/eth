# Trade Simulation Examples

These examples exercise `tx_processor::trade_simulation` against known DEX pool
types. Keep durable examples under `viability/`; keep ad hoc but useful
debugging CLIs under `probes/`.

## Durable Viability Examples

| Example | Purpose |
| --- | --- |
| `can_buy_sell_common_tokens_uniswap_v2` | Known Uniswap V2 token set and expected behavior. |
| `can_buy_sell_common_tokens_multi_v2` | V2-style router protocols beyond Uniswap: SushiSwap, PancakeSwap, ShibaSwap, Fraxswap. |
| `can_buy_sell_common_tokens_uniswap_v3` | Uniswap V3 fee-tier viability checks. |
| `can_buy_sell_common_tokens_pancakeswap_v3` | PancakeSwap V3 pool derivation and viability checks. |
| `can_buy_sell_common_tokens_uniswap_v4` | Uniswap V4 Universal Router viability using `UniswapV4PoolConfig`. |
| `usdc_to_dai_uniswap_v3` | Denom-token pipeline for USDC -> DAI on Uniswap V3. |
| `erc20_pool_tax_demo` | Tax calculation from a round-trip pool simulation. |
| `erc20_pool_block_range_analysis` | Historical viability over a block range. |

## Diagnostic Probes

| Example | Purpose |
| --- | --- |
| `probe_pool_buy_sell` | Parameterized buy -> approve -> sell probe over one or more amounts. |
| `probe_sell_swap` | Sell-only probe for router and Universal Router paths. |
| `probe_uniswap_v4_pool` | Parameterized Uniswap V4 pool-manager probe. |
| `check_token_server_uniswap_v3_pools` | Compare server-provided V3 pools against live trade simulation. |

## Running

```bash
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example can_buy_sell_common_tokens_uniswap_v2
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example can_buy_sell_common_tokens_uniswap_v4
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example probe_sell_swap -- --help
```

Examples should either use fixed fixture addresses for repeatable regression
coverage or expose CLI arguments when the purpose is investigation.
