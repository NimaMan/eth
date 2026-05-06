# manager

Block-level token orchestration.

This folder corresponds to Python modules under `token_manager` and `token_builder`.

## Responsibilities

- Consume processed Rust blocks and update tracked token state.
- Route events to token, pool, state, health, and network modules.
- Maintain live token caches.
- Provide a stable API for future PyReth bindings.

## Boundaries

- This module must not process raw blocks or raw transactions.
- Historical and live inputs should come from Rust `tx_processor` / `pyreth.block_processor`.
- Redis subscriptions and publication should remain separate from core token state transitions.

## Port Status

1. `BlockTokenProcessor` consumes `tx_processor::ProcessedBlock`, applies transactions in block order, and reports token/pool updates.
2. `TokenRegistry` owns tracked token storage and lookup.
3. `ProcessedTokenUpdateRouter` routes processed transaction events into token and Uniswap V2 pool state.
4. `TokenMetadataProvider`, `UniswapV2PoolMetadataProvider`, and `RethChainDiscoveryProvider` hydrate token and pool metadata when direct chain reads are needed.
5. `TokenStateCache` owns token/pool address indexing and cache status tracking.
6. `TokenStateBuilder` rebuilds a token from processed Rust transactions or processed blocks.
7. Live token processor orchestration should be added after historical parity is established.
