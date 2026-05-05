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

## First Port Targets

1. `block_token_processor.py` as a Rust processor over `ProcessedBlock`.
2. `live_tokens_cache.py`.
3. `live_token_builder.py` behavior that is not Python-specific.
4. Live token processor orchestration after historical parity is established.
