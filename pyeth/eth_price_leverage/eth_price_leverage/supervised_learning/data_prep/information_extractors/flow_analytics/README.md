# Flow Analytics Extractors

Summaries derived from `tx_processor`, `pyreth`, and related analytics crates capturing capital movement:
- net ETH/stablecoin flows between major entities
- large transfer detection and clustering
- smart-money balance changes
- directional flow imbalances over configurable windows

Outputs should be timestamped to block numbers and respect schema constraints for reproducibility.
