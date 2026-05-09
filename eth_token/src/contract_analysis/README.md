# contract_analysis

Contract-level evidence extracted from tracked token state.

This module is intentionally not a scam oracle. It records what the token
contract appears to expose or do, then higher layers decide how to bucket or
display that evidence.

## Boundaries

- `contract_analysis` owns structured token-contract evidence.
- `chain_metadata` and `reth_chain_query` own raw chain reads and view calls.
- `health` can turn evidence into policy decisions later.
- `eth_token_server` only exposes reports through API DTOs.
- `token_lab` consumes reports to choose investigation cases.

## Current Scope

The first pass analyzes the already-indexed `ERC20Token` state:

- metadata completeness and basic sanity,
- declared supply versus transfer-derived minted supply,
- ownership/control surface observed from events,
- transfer and approval surface,
- pool/trading surface,
- behavior flags such as hidden mint evidence or raw trading events without
  pool-derived trading.

Future bytecode and view-call analyzers should feed the same report shape
without making this module depend on server or lab code.
