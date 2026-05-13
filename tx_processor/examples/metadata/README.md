# Metadata Examples

These examples focus on metadata reads that belong near `tx_processor` because
they depend on replaying or sharing DB-backed simulator state.

## Examples

### `token_metadata_with_prior_tx`

Loads a processed contract-creation transaction, replays it before a metadata
call, and queries the freshly created token contract.

```bash
cargo run -p tx_processor --example token_metadata_with_prior_tx -- \
  --tx-hash 0x... --contract 0x... --metadata-block <block-1>
```

### `denom_token_metadata`

Fetches metadata for every denomination token listed in `reth_chain_query`.

```bash
cargo run -p tx_processor --example denom_token_metadata -- --block <block_number>
```

`RETH_DATADIR` should point at the local Reth datadir unless the example exposes
an explicit datadir flag.
