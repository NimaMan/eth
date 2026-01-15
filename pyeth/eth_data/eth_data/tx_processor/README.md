# Ethereum Transaction Processor

This package turns raw Ethereum transaction artifacts into a canonical, machine-scale `ProcessedTransaction` record. The transformation combines calldata, receipts, traces, and optional state diffs into a single structure that downstream systems can consume without re-querying the chain. The implementation mirrors the schema documented in `eth_data/tx_processor/data_models/README.md`, keeps every quantity in integer base units, and stays byte-compatible with the Rust `ProcessedTransaction` exposed through PyReth.

## Processing Pipeline

1. **Acquisition** – callers obtain the transaction, receipt, trace, and (optionally) state diff using `TransactionDataFetcher`/`TransactionBatchDataFetcher` or custom RPC logic.
2. **Fee Normalisation** – `_extract_transaction_fees` converts gas data into integer wei values and annotates the protocol type (legacy / EIP-1559 / EIP-2930).
3. **Log Decoding** – `TransactionLogProcessor.process_logs` classifies receipt logs into typed dataclasses (ERC transfers/approvals, Uniswap V2/V3/V4, Permit2, ownership toggles, etc.) while checksum casing addresses up front.
4. **Trace Interpretation** – when required, `TransactionTraceProcessor.process_trace` walks the `callTracer` tree to emit `InternalTransaction` entries (checksum addresses, integer wei amounts, propagated failures, contract-creation address fallbacks).
5. **Participant Aggregation** – `_to_checksum_address` + `extend_unique_addresses` merges participants from the transaction, logs, traces, and synthetic events into checksum sets (`unique_addresses`, `erc20_contracts`).
6. **Classification & Actions** – `EthTransactionClassifier` supplies the coarse `tx_type`; `TransactionActionIdentifier` refines it into higher-level actions (swap, add liquidity, enable trading, etc.).
7. **Synthetic Events** – `_add_tx_type_events` injects derived events (e.g., contract creation summaries, trading toggles) using the canonical dataclasses.
8. **Bribe & Balance Metrics** – `_get_bribe_amount` summarises builder tips; `AddressBalanceChangeCalculator` optionally aggregates per-address deltas.
9. **Materialisation** – `process_transaction` (and its async wrapper) assemble a `ProcessedTransaction` with checksum addresses, boolean `status`, integer amounts, and structured event collections ready for `to_dict()` / `to_json()` serialisation.

## Canonical Output Highlights

- `status` is a boolean (`True` for success, `False` for revert).
- `block_number`, `tx_index`, `nonce`, and all fee fields are integers (`_normalize_int` handles hex strings automatically).
- All address sets (`unique_addresses`, `erc20_contracts`) contain checksum strings; `None` is stripped before materialisation.
- `internal_transactions` expose checksum participants, integer wei amounts, gas usage, and `trace_type`/`call_type` metadata.
- `eth_transfers` is populated only for simple ETH sends with no contract execution.
- The async entry point defers to the synchronous implementation once data acquisition completes—no duplicate transformation code.
- Cross-runtime serialisation: `ProcessedTransaction.to_dict()` returns the canonical Python structure with full-precision integers; `to_json()` leverages `orjson` for fast encoding once the structure is assembled. When we publish to transports with numeric limits (e.g. RabbitMQ JSON fan-out) we run the shared `transaction_serializer` helper that stringifies integers outside signed 64-bit range so the full U256 magnitude survives the hop.

## Testing

- `tests/tx/tx_processor/tx_log_processor/` exercises log decoding with both synthetic payloads (`test_parsers_units.py`) and real mainnet receipts (`test_process_logs_mainnet.py`).
- `tests/tx/tx_processor/tx_trace_processor/` covers trace traversal using crafted fixtures and live transactions (failed contract creation, contract deployment).
- The historical regression fixtures under `tests/tx/tx_processor/test_raw_tx_processor/` continue to validate end-to-end transaction processing for representative scenarios (swaps, approvals, tax toggles, bribe detection).
- `tests/tx/tx_processor/test_python_pyreth_alignment.py` runs the same transaction through the Python pipeline and the Rust PyReth processor, asserting that every field of the canonical schema matches. This guards the cross-language contract.
- `tests/tx/tx_processor/test_tx_processor/test_live_block_processor_publish.py` verifies that the publishing path can serialise the canonical transactions (and recent blocks) without dropping magnitudes, including >64-bit integers.

## Operational Notes

- Provide `debug_traceTransaction` (or `debug_traceBlockByNumber`) access when trace-derived data is required.
- Prefer `eth_getBlockReceipts` (Reth/Erigon/Nethermind) for batch pipelines to reduce RPC chatter.
- All monetary quantities remain in wei; presentation layers should handle unit conversion for human-readable output.
- Use the shared normalisers (`transaction_serializer`, `alert_serializer`) when shipping data out of process so non-canonical transports still convey the machine-scale integers losslessly.

By adhering to these conventions, `TransactionProcessor` produces stable, cross-language transaction representations that line up with the crate’s Rust counterpart.
