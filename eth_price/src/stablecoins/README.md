# Stablecoin Snapshot Reader

`StablecoinPriceReader` focuses exclusively on quoting ETH against the core USD
stablecoins across the venues we already support (Uniswap V2/V3, SushiSwap,
Curve). The universe of `(token, denom)` pairs is defined once inside
`reth_chain_query/src/common_addresses/dex_token_denom_pairs.rs`, so the price
reader simply iterates those specs and asks the AMM readers for each label.

Key traits:

- **Deterministic pairs** – every entry in `dex_token_denom_pairs` provides a
  label, protocol, and pool address. The reader does not invent “actions” or
  routes on the fly anymore.
- **Snapshot only** – prices are taken from pool state (or Curve view calls) at
  the selected block. No swap simulations or “buy actions” live here; those
  will move to the future `eth_agent` crate.
- **Per-protocol maps** – helper methods (`eth_usdc`, `eth_usdt`, `eth_dai`,
  `eth_stablecoin_prices`) return simple `HashMap<String, PriceData>` results so
  downstream systems can consume them directly.

If you need executable swap simulations or higher-level “actions”, build them
inside the upcoming `eth_agent` repo using `BuySwapReader`. `eth_prices`
stays focused on deterministic pair pricing.
