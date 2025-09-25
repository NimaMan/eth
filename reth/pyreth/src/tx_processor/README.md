Interoperability Plan: ProcessedTransaction between Rust and Python

Overview

- Goal: Ensure Rust and Python produce the same ProcessedTransaction from the blockchain, and use one shared data structure when exchanging results between the two languages. Python already fetches on‑chain data and builds processed transactions; Rust does the same and also produces simulation results. Both must conform to a single schema so we can pass data seamlessly either way.
- Scope: Transaction processing and simulation results (including pool buy→approve→sell) flow across the boundary as ProcessedTransaction objects (Rust → Python for simulations most of the time). Downstream analytics in Python operate on the same schema as the existing Python dataclasses.

Design Principles

- Single schema contract: Both Rust and Python implementations conform to the exact same ProcessedTransaction shape (field names, types, and nested structures). Either side may be the producer (Python for historical loads; Rust for high‑throughput and simulations).
- Lossless transport: Large integers (U256) and binary data are converted to safe string/hex types; no float down‑casting.
- Thin bindings: pyo3 classes expose Rust data; no business logic in bindings (e.g., tax math, trading enabled checks) — those stay in Rust.
- Backward‑compatible schema: The dict shape Python receives matches eth_data/eth_data/tx_processor/data_models/txn_models.py dataclasses.

Data Model Contract

- Canonical Rust type: tx_processor::tx_processor::data_models::ProcessedTransaction
  - Fields: hash, block_number, block_timestamp, txn_index, from_address, to_address, contract_address, value, status, nonce, txn_type, actions, fees, bribe_amount, unique_addresses, erc20/721/1155_contracts, eth/erc20/erc721/erc1155 transfers, internal_transactions, uniswap_v2/v3/v4 events, permit2, other_events, address_balance_changes, latest_states, input
- Python target type: eth_data.tx_processor.data_models.txn_models.ProcessedTransaction
- Mapping rules:
  - Addresses: EIP‑55 checksum strings (0x‑prefixed)
  - Hashes: 0x‑prefixed hex strings
  - Amounts (U256): decimal strings (not float), e.g. "1000000000000000000"
  - Bytes/input: 0x‑prefixed hex string
  - Sets (unique_addresses, contract sets): Python set[str]; when serializing to dict/JSON, may become list[str]
  - Nested events/structs: map field‑for‑field to Python dicts matching existing dataclasses (ERC20Transfer, InternalTransaction, UniswapV2Swap, etc.)
  - Address balance changes: exposed via ProcessedTransaction.address_balance_changes in Rust; Python can continue to compute derived metrics from there as needed

Binding Surface (current and planned)

- Tx processing entry points (rust/pyreth/src/python/tx_processor/py_tx_processor.rs):
  - TxProcessor.process_transaction_from_hash_with_simulation(hash: str) -> PyProcessedTransaction
    - Loads tx by hash and SIMULATES it at block-1 to compute balance changes, internal traces, etc.
  - TxProcessor.load_transaction_from_hash_db_only(hash: str) -> PyProcessedTransaction
    - Loads and decodes tx from DB only (no simulation). Produces decoded events/fees/metadata, but no
      balance changes or internal traces. Use this when you need fast event decoding only.
  - TxProcessor.simulate_unsigned_transaction(...) -> PyProcessedTransaction
    - Pure simulation for a synthetic tx (e.g., for buy→approve→sell checks).
  - Batch helpers returning list[PyProcessedTransaction]
- Pool viability and swap simulators (rust/pyreth/src/python/simulator/*):
  - PoolBuySellSimulator.check_uniswap_v2_pool(...)
  - PoolBuySellSimulator.check_uniswap_v3_pool(...)
  - PoolBuySellSimulator.check_sushiswap_pool(...)
  - Planned upgrade: results include buy_transaction, approve_transaction, sell_transaction as PyProcessedTransaction for full transparency on each step.

PyProcessedTransaction utility methods (to add)

- to_dict() -> dict: returns a dict that conforms exactly to Python dataclasses in eth_data/eth_data/tx_processor/data_models/txn_models.py. Field names and shapes match. Useful when you want a pure‑Python object graph.
- to_json() -> str: canonical JSON with the same schema (decimal strings for U256, checksum addresses, hex for bytes).
- as_python_dataclass() -> eth_data.tx_processor.data_models.txn_models.ProcessedTransaction: convenience constructor that calls ProcessedTransaction.from_dict on the dict above.

Why dict/json interop in addition to pyo3 classes?

- Seamless integration with existing Python code and tests that expect dataclasses and dicts.
- Stable contract for non‑Python consumers (notebooks, services) when needed.
- Zero logic duplication: conversion is mechanical; analytics remain in Rust or in existing Python modules that operate on the canonical dict.

Trading Viability (Buy→Approve→Sell) Flow

- Rust orchestrates the full sequence and returns:
  - Flags: can_buy, can_approve, can_sell, is_tradeable, buy_tax_percent, sell_tax_percent
  - Transactions: buy_transaction, approve_transaction, sell_transaction (ProcessedTransaction)
- Python binding returns a PyPoolViabilityResult and (planned) the three PyProcessedTransaction objects so Python can inspect and/or persist them using the same schema.
- Python→Rust: In code paths like `rust/pyreth/src/python/simulator/pool_buy_sell_simulator.rs`, Python provides
  input parameters (or a seed tx) and Rust performs the simulations and returns results as PyProcessedTransaction
  instances. This complements the common Rust→Python path used for high‑throughput decoding from the Reth DB.
- This preserves a single workflow in Python whether a tx came from chain or simulation.

Examples (intended usage from Python)

- Single tx from hash (with simulation):
  - ptx = TxProcessor().process_transaction_from_hash_with_simulation("0x...")
  - py_dict = ptx.to_dict()  # canonical, lossless schema (U256 as decimal strings)
  - py_dc = ptx.as_python_dataclass()  # optional convenience

- Single tx from hash (DB‑only, no simulation):
  - ptx = TxProcessor().load_transaction_from_hash_db_only("0x...")
  - py_dict = ptx.to_dict()  # events/fees present; state_changes will be empty

- Simulate buy→approve→sell viability:
  - res = PoolBuySellSimulator().check_uniswap_v2_pool(token, pool)
  - if res.can_buy: inspect res.buy_transaction.to_dict()  # planned addition

Performance & Compatibility

- Use DB‑only for fast, large‑scale event decoding; use simulation paths when you need balance deltas and internal traces.
- validate_rust_python_compatibility.py in rust/tx_processor/examples/python can compare field‑by‑field outputs between Rust and Python processors on the same tx hash.
- Keep both ProcessedTransaction schemas aligned; add CI checks on a few canonical transactions.

Implementation Notes

- PyProcessedTransaction.to_dict() returns a schema that matches Python dataclasses exactly; large integers are preserved as strings.
- Numeric‑looking strings are never down‑cast to floats in the binding conversion layer to avoid precision loss.
- Extend PyPoolViabilityResult to include buy/approve/sell transactions (as PyProcessedTransaction) and optionally prior_tx.
- Provide batch/streaming versions for high‑volume use cases (yield iter of PyProcessedTransaction, or Arrow/IPC in the future).
- Document field‑by‑field schema mapping in docstrings and keep it in lock‑step with Rust/Python models.

Non‑Goals

- Re‑implement analytics in Python that already exist in Rust.
- Add business logic to bindings; pyo3 layer remains a transport/glue layer only.
