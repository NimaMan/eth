# Ethereum Transaction Processor

Transaction processing is now owned by the Rust `tx_processor` crate exposed
through PyReth.  The Python package keeps compatibility entry points so older
imports still work, but those classes delegate to Rust:

- `TransactionProcessor` -> `PyReth().tx_processor()`
- `TransactionBatchProcessor` -> `PyReth().tx_processor()` and
  `PyReth().processed_tx_provider()`
- `TransactionDataFetcher` -> hash-only compatibility facade; Rust loads
  receipts, traces, logs, and state from Reth

The Python data model modules remain because downstream alerting, token, and
serialization code still import the canonical field names.  They are schema
definitions, not a second processing implementation.

Retired Python internals such as the log parser, trace parser, transaction
classifiers, action identifier, and balance-change calculator have been
removed.  Use the processed transaction returned by PyReth instead, including
`actions`, event lists, `unique_addresses`, `erc20_contracts`, and
`address_balance_changes`.

## Operational Notes

- Build/install PyReth before importing these wrappers.
- Set `PYRETH_DATADIR` when the Reth database is not at PyReth's default path.
- In this conda environment the installed PyReth wheel requires the newer system
  C++ runtime:

```bash
LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libstdc++.so.6
```

All block and transaction processing should go through Rust/PyReth.  Python
callers should treat this package as compatibility and schema glue only.
