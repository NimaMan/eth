# Leverage State Extractors

Hydrate leverage and liquidation context from existing leverage monitors (`eth_price_leverage` Rust crate, DeFi protocol readers, CEX connectors). Information may include:
- aggregated open interest and funding data
- collateral ratios and liquidation buffers per venue
- recent liquidation events and magnitudes

Extractors should specify provenance (on-chain vs off-chain) and normalize units for downstream consumers.
