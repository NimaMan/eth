# Information Extractors

Reader modules that hydrate schema-defined information directly from node-native crates (`eth_prices`, `tx_processor`, `mempool_processor`, etc.). Each extractor should expose deterministic APIs such as `hydrate_at_block` or `hydrate_range` returning typed payloads aligned with the schema.

Subdirectories group extractors by domain:
- `prices/` — AMM, oracle, and CEX pricing information.
- `leverage_state/` — on-chain and off-chain leverage metrics, liquidation buffers.
- `flow_analytics/` — token/ETH flow summaries, large transfers, balance shifts.
