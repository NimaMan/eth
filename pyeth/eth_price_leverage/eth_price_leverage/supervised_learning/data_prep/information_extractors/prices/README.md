# Prices Extractors

Implementations that read deterministic ETH pricing information from the Rust `eth_prices` crate and related helpers. Typical outputs include:
- spot and mid prices per venue
- liquidity-aware price gradients
- historical volatility snippets derived from AMM state
- oracle reference prices for anchoring targets

Each extractor should document required configuration (datadir, block range, venues) and produce typed structures referenced by the schema.
