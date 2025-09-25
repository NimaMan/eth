# Provider Examples

Examples demonstrating how to use the PyReth processed-transaction providers from Python.

Run any script with the repository root on `PYTHONPATH` and the Reth database location exported via `PYRETH_DATADIR` if you use a non-default path.

Scripts:

- `process_latest_block.py` – process the latest canonical block and print a short summary.
- `fetch_address_transactions.py` – warm the address cache for a block range and list the processed transactions touching the address.
- `fetch_token_transactions.py` – similar to the address script but scoped to an ERC20 token contract.
- `token_processed_transactions_example.py` – concrete example targeting token
  0xc8Ab73b7EaeE2FD3C858dcDB22fB03433A7aeB9F with an adjustable block window.
