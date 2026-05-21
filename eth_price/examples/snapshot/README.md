# Snapshot Readers

These examples demonstrate one-shot price reads that stay in "machine precision" all the way
through: AMM reserves are pulled straight from the Reth database and more complex pools use
local view-call simulation. Each example can be executed with `cargo run --example <name>`.

| Example name | What it shows |
|--------------|---------------|
| `uniswap_v2_price_feed` | Reads packed reserves from the canonical WETH/USDC pool and converts them into a `RawChainPrice`. |
| `uniswap_v3_price_feed` | Extracts `sqrtPriceX96` from storage, builds a precise quote for multiple fee tiers, and prints the raw ratio. |
| `sushiswap_price_feed` | Verifies Uniswap V2–compatible state layout and emits the same precise price struct. |
| `curve_price_feed` | Calls the Curve pool contracts locally (via tx-simulator) to recover pool balances and quote ETH/stable ratios. |
| `balancer_price_feed` | Uses the Balancer vault view methods to gather pool weights and compute a weighted price snapshot. |
| `pancakeswap_v3_price_feed` | Demonstrates another V3-style pool (different chain originally) while preserving integer math. |
| `dodo_price_feed` | Runs the PMM view helpers inside the simulator and surfaces the raw integer amounts that drive the quote. |
| `fraxswap_price_feed` | Reads the V2-style reserves for Fraxswap’s TWAMM pools. |
| `chainlink_price_feed` | Executes the aggregator’s `latestRoundData` view call locally so the resulting `PriceData` carries the chain-provided integer answer. |
| `multi_venue_snapshot` | Aggregates every configured snapshot reader, prints raw ratios, and reports summary statistics (median, averages, spreads). |

All of these commands assume a synced Reth mainnet datadir at
`/home/nima/.local/share/reth/mainnet`. If your database lives elsewhere, set the
`RETH_DATA_DIR` environment variable in the example or update the hard-coded path before
running.
