# erc20

ERC-20 token-level models and helpers.

This folder corresponds to Python modules such as:

- `erc20_token.py`
- `token_snapshot.py`
- `token_chain_data_fetcher.py`

## Responsibilities

- Represent a tracked ERC-20 token and its static metadata.
- Produce token snapshots for downstream consumers.
- Provide token-level chain data helpers through Rust `reth_chain_query` or PyReth-backed APIs.
- Normalize token decimals, symbols, addresses, supply, balances, and snapshot fields.

## Boundaries

- Pool-specific behavior belongs in `pools`.
- Transfer/control-address state belongs in `state`.
- Health scoring belongs in `health`.
- This module should not perform block or tx processing.

## First Port Targets

1. Token metadata and snapshot structs.
2. Python `ERC20Token` state fields that are independent of pool behavior.
3. Snapshot serialization compatible with current Python consumers.
