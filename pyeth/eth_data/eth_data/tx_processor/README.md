# Ethereum Transaction Processor

## Overview
The transaction processor is the canonical path for transforming raw Ethereum transaction artifacts into rich analytical objects. It unifies transaction metadata, receipt logs, execution traces, and optional state diffs into a standardized `ProcessedTransaction` model. Downstream systems—portfolio management, risk detection, MEV analysis, auditing—consume this model to reason about what happened in a transaction without re-querying chain data.

## Conceptual Pipeline
1. **Acquisition** – transactions, receipts, traces, and (optionally) state diffs are fetched from the node or via simulation helpers in this package.
2. **Fee Normalization** – gas usage, effective prices, and protocol types (legacy, EIP-1559, EIP-2930) are converted into a unified `TransactionFees` structure.
3. **Log Decoding** – receipt logs are parsed into typed dataclasses for ERC transfers, Uniswap V2/V3/V4 activity, ownership changes, trading toggles, approvals, and more.
4. **Trace Interpretation** – `callTracer` output is traversed to expose internal ETH transfers, contract creations, fallback calls, and failure chains.
5. **Participant Indexing** – unique addresses and involved token contracts are deduplicated into fast-access sets.
6. **Classification & Actions** – function signatures, known contract maps, and decoded logs feed the transaction classifier and action identifier to assign high-level intents (swap, add liquidity, trading enablement, approval, etc.).
7. **Derived Metrics** – the processor computes builder bribes, synthesizes inferred events (e.g., contract creation), and, when enabled, aggregates per-address balance deltas.
8. **Materialization** – all artifacts are assembled into a `ProcessedTransaction`, ready for serialization (`to_dict` / `to_json`) and downstream consumption.

## Data Sources
- **Transaction payloads** via `eth_getTransactionByHash`.
- **Receipts** via `eth_getTransactionReceipt` or `eth_getBlockReceipts`.
- **Execution traces** via `debug_traceTransaction` or `debug_traceBlockByNumber` (using `callTracer`).
- **State diffs / simulations** via `trace_call` or `debug_traceCall` (optional enrichment).
- **Mempool contents** via `txpool_content` for pre-inclusion inspection.

## Core Components
- `tx_processor.py` – orchestrates the end-to-end transformation, manages trace requirements, and injects synthetic events plus balance deltas.
- `tx_batch_processor.py` – coordinates block-level or arbitrary batch processing using asyncio.
- `tx_data_fetcher.py` – handles JSON-RPC acquisition in both synchronous and batched asynchronous modes.
- `tx_log_processor.py` – classifies receipt logs with shared ABI/topic registries and produces typed event dataclasses.
- `tx_trace_processor.py` – walks trace trees to create normalized internal transaction records.
- `address_balance_change_calculator.py` – aggregates ERC-20 transfers and internal ETH flows into per-address net movements with denomination-aware thresholds.
- `tx_type_classifier.py` & `tx_action_identifier.py` – provide transaction type labelling, protocol classification, and action tagging logic.
- `tx_simulator.py` & `mempool/mempool_tx_state_diff_processor.py` – run simulations for pending transactions to extract traces/state diffs ahead of block inclusion.
- `mempool/mempool_data_fetcher.py` – reads pending and queued transactions directly from node mempools.

## `ProcessedTransaction` Data Model

### 1. Core Transaction Details

| Field | Type | Description |
| :--- | :--- | :--- |
| `hash` | `str` | Unique 32-byte transaction hash. |
| `block_number` | `int` | Block number containing the transaction (0 for pending). |
| `block_timestamp` | `int` | Unix timestamp of the block. |
| `tx_index` | `int` | Position of the transaction within the block. |
| `from_address` | `ChecksumAddress` | Sender address. |
| `to_address` | `Optional[ChecksumAddress]` | Receiver address, or `None` if contract creation. |
| `contract_address` | `Optional[ChecksumAddress]` | Address of a newly created contract. |
| `value` | `np.float64` | ETH value transferred (expressed in Ether for readability). |
| `status` | `str` | Execution outcome (`"success"` / `"failure"`). |
| `nonce` | `int` | Sender nonce. |
| `input` | `str` | Calldata hex string. |

### 2. Classification & Fees

| Field | Type | Description |
| :--- | :--- | :--- |
| `tx_type` | `str` | High-level classification (Ether transfer, approval, contract interaction, swap, etc.). |
| `actions` | `List[str]` | Higher-level behaviours inferred from decoded events (swap, add liquidity, trading enablement, ownership change). |
| `fees` | `TransactionFees` | Aggregated gas metrics: effective price, gas used, total fee (in ETH), protocol type, optional max fee/priority fee. |
| `bribe_amount` | `float` | ETH paid to known builder/validator recipients derived from internal transfers. |

### 3. Participant Sets

| Field | Type | Description |
| :--- | :--- | :--- |
| `unique_addresses` | `Set[ChecksumAddress]` | All EOAs and contracts touched by the transaction (inputs, logs, traces, inferred events). |
| `erc20_contracts` | `Set[ChecksumAddress]` | ERC-20 contracts encountered in logs or inferred events. |
| `erc721_contracts` | `Set[ChecksumAddress]` | ERC-721 contracts referenced in the transaction. |
| `erc1155_contracts` | `Set[ChecksumAddress]` | ERC-1155 contracts referenced in the transaction. |

### 4. Transfer & Event Collections

| Collection | Description |
| :--- | :--- |
| `eth_transfers` | Native ETH transfers captured for simple value sends without contract interaction. |
| `erc20_transfers` | ERC-20 `Transfer` events decoded from receipts. |
| `erc721_transfers` | ERC-721 transfers (NFT movements). |
| `erc1155_transfers` | ERC-1155 single/batch transfers. |
| `approvals`, `erc721_approvals` | Token approvals across ERC standards. |
| `mints`, `burns`, `deposits`, `withdraws` | Liquidity provisioning/removal and wrapper token flows. |
| `pair_events`, `owner_events` | DEX pair creations and ownership transitions. |
| `trading_enabled_events`, `trading_disabled_events` | Trading toggle events, including synthetic inferences from classification. |
| `contract_creation_events` | Synthetic events summarizing newly created contracts (metadata placeholders until enrichment). |
| `other_events` | Recognized but uncategorized events for future handling. |

### 5. Uniswap & Permit Coverage

| Ecosystem | Collections |
| :--- | :--- |
| Uniswap V2 | `uniswap_v2_syncs`, `uniswap_v2_swaps` |
| Uniswap V3 | `uniswap_v3_pools`, `uniswap_v3_initializations`, `uniswap_v3_mints`, `uniswap_v3_burns`, `uniswap_v3_swaps`, `uniswap_v3_positions`, `uniswap_v3_increases`, `uniswap_v3_decreases` |
| Uniswap V4 | `uniswap_v4_initializes`, `uniswap_v4_modifies`, `uniswap_v4_swaps`, `uniswap_v4_donates`, `uniswap_v4_protocol_fee_updates`, `uniswap_v4_dynamic_lp_fee_updates`, `uniswap_v4_protocol_fee_controller_updates`, `uniswap_v4_balance_deltas` |
| Permit2 | `permit2_events` captured from pool manager interactions. |

### 6. Internal Execution (Trace-Derived)

| Field | Type | Description |
| :--- | :--- | :--- |
| `internal_transactions` | `List[InternalTransaction]` | Flattened call tree entries (CALL, DELEGATECALL, STATICCALL, CREATE, CREATE2) with from/to, value (wei), gas, depth, and error context. |
| `address_balance_changes` | `Dict[str, Any]` | Optional per-address net movements across known currencies and unknown-token contract addresses. |
| `latest_states` | `Dict[str, Any]` | Reserved for post-state snapshots when simulation or prestate tracers are available. |

### 7. Serialization Utilities
- `to_dict()` converts nested dataclasses and sets into JSON-friendly structures.
- `to_json()` produces a compact JSON string for storage or message passing.

## Derived Metrics and Optional Enrichments
- **Builder bribe detection** using known fee recipient sets.
- **Address balance deltas** computed with denomination-aware thresholds and token metadata.
- **Synthetic trading enable/disable events** for tax/toggle detection.
- **Protocol-specific heuristics** for Uniswap, Permit2, and emerging AMM ecosystems.

## Operational Considerations
- Nodes must expose tracing endpoints (`debug_trace*`, `trace_call`) to populate internal transactions and state diff data.
- `eth_getBlockReceipts` (or equivalent) is strongly recommended for batch processing; ensure node compatibility (Reth, Erigon, Nethermind).
- `DENOM_ADDRESSES`, `ERC20_TOKEN_DECIMALS`, and `fee_recipients` lists underpin denomination resolution and bribe detection; keep them current as new tokens or builders appear.
- ETH amounts inside `ProcessedTransaction` are presented in Ether for readability; accompanying transfer objects carry precise wei values for accounting.

## Future Enhancements

| Area of Enhancement | Description | Importance |
| :--- | :--- | :--- |
| Detailed State Diffs | Replace generic state dictionaries with structured balance/nonce/storage diff objects. | **High** |
| Pre-State Tracing | Capture pre-transaction state via `prestateTracer` for MEV and what-if simulations. | **High** |
| EIP-4844 Fields | Surface blob-related metrics (`blobGasUsed`, `blobGasPrice`, hash lists) introduced with Dencun. | **High** |
| Token Metadata Enrichment | Continuously populate contract metadata (symbol, decimals, name, total supply) from on-chain or trusted sources. | **Medium** |
| Transaction Signatures | Persist `v`, `r`, `s` for wallet analytics and forensic tooling. | **Medium** |
| EIP-2930 Access Lists | Parse optional access lists to observe gas-optimizing behaviour. | **Low** |
| Contract Bytecode Snapshots | Cache deployed code for similarity analysis and unverified contract detection. | **Low** |

## Related Artifacts
- `scripts/ipy/tx_processor.ipynb` illustrates the processor output and typical analyses.
- `eth_portfolio_manager` and other live trading/backtesting services consume `ProcessedTransaction` objects for signal generation.

