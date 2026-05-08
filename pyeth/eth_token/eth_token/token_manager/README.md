# Token Manager Code Map

This directory now keeps the Python block-level token mutation code that is still useful for historical compatibility and tests. The Python live token tracker, Redis subscriber, and Redis snapshot publisher were removed; live token state is owned by the Rust `eth_token`, `eth_live_feed`, and `eth_live_state` crates.

## Module Roles

| File | Purpose |
| ---- | ------- |
| `block_token_processor.py` | Deterministic per-block mutation engine for ERC-20 tokens. Walks transactions sequentially, handles contract creation instantly, mutates existing tokens in order, and refreshes `LiveTokensCache`. |
| `live_tokens_cache.py` | LRU cache of active `ERC20Token` objects with pool-to-token mapping, optional PnL persistence, and eviction policies. |

## Historical Flow

```text
pyreth block processor
  -> ProcessedBlock / processed transactions
  -> BlockTokenProcessor.process_block_tokens
  -> ERC20Token and pool state mutation
  -> updated_tokens for caller-side inspection
```

## Boundaries

- Do not add Python Redis block subscribers.
- Do not add Python live token snapshot publishers.
- Do not add new Python live tracker orchestration.
- Keep new live-feed/runtime work in Rust.

## Rust Replacements

- `reth/eth_token`: token and pool state transitions.
- `reth/alpha/live/feed`: live confirmed-chain token feed orchestration.
- `reth/alpha/live/state`: live-state schemas, keys, and store traits.
