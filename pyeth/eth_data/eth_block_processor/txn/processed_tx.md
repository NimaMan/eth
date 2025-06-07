# The Processed Ethereum Transaction Data Model

## 1. Objective

The `ProcessedTransaction` is a comprehensive Python dataclass that represents a fully dissected Ethereum transaction. Its purpose is to provide a single, rich, and standardized object containing not just the raw transaction data, but also the results of its execution, including decoded event logs, internal ETH transfers, classified actions, and detailed fee information. This object is the primary output of the `eth_block_processor` and serves as the foundation for any downstream analysis, such as PnL calculations, fund flow tracking, or scam detection.

## 2. Data Sources

The data within a `ProcessedTransaction` is aggregated from three primary sources, which are fetched and processed by the system:

1.  **Transaction Data**: The raw transaction payload itself (`eth_getTransactionByHash`). This includes sender/receiver, value, input data, and nonce.
2.  **Transaction Receipt**: The result of the transaction's execution (`eth_getTransactionReceipt`). This provides the status (success/fail), gas usage, and a list of raw event logs.
3.  **Transaction Trace**: A detailed, step-by-step record of the EVM execution (`debug_traceTransaction`). This is crucial for uncovering internal ETH transfers and nested contract calls that are not visible in the receipt logs.

## 3. `ProcessedTransaction` Data Structure

The following sections detail every field available in the `ProcessedTransaction` dataclass.

---

### **3.1 Core Transaction Details**

These fields represent the fundamental properties of the transaction.

| Field | Type | Description |
| :--- | :--- | :--- |
| `hash` | `str` | The unique 32-byte hash identifying the transaction. |
| `block_number`| `int` | The block in which this transaction was included. |
| `block_timestamp`| `int` | The Unix timestamp of the block. |
| `txn_index` | `int` | The transaction's position within the block (e.g., 0, 1, 2...). |
| `from_address`| `ChecksumAddress` | The address of the account that sent the transaction. |
| `to_address` | `Optional[ChecksumAddress]` | The recipient's address. Can be `None` for contract creation. |
| `contract_address`|`Optional[ChecksumAddress]`| If the transaction created a contract, this is its address. |
| `value` | `np.float64` | The amount of ETH transferred, in Ether (not Wei). |
| `status` | `str` | The outcome of the transaction: `'success'` (1) or `'failure'` (0). |
| `nonce` | `int` | The sender's transaction count. |
| `input` | `str` | The data sent with the transaction (e.g., function call `0x...`). |

---

### **3.2 Classification and Fees**

These fields provide high-level context and cost information.

| Field | Type | Description |
| :--- | :--- | :--- |
| `txn_type` | `str` | A high-level classification (e.g., "Swap", "Transfer", "Contract Creation"). |
| `actions` | `List[str]` | A list of specific, identified actions (e.g., "Swap ETH for Token"). |
| `fees` | `TransactionFees` | A nested dataclass containing `gas_price`, `gas_used`, and total `txn_fee`. |
| `bribe_amount`| `float` | The amount of ETH paid directly to the block builder (extracted from traces). |

---

### **3.3 Address and Contract Sets**

These fields aggregate all unique parties involved in the transaction.

| Field | Type | Description |
| :--- | :--- | :--- |
| `unique_addresses` | `Set[ChecksumAddress]` | A set of all unique EOA and contract addresses involved in any way. |
| `erc20_contracts` | `Set[ChecksumAddress]` | A subset of `unique_addresses` that are specifically ERC-20 token contracts. |

---

### **3.4 Decoded Event Logs**

The `TransactionProcessor` decodes the raw logs from the receipt into structured dataclasses, which are then stored in categorized lists.

| Field | Type | Description |
| :--- | :--- | :--- |
| `eth_transfers` | `List[ETHTransfer]` | Native ETH transfers (derived from transaction `value`). |
| `erc20_transfers` | `List[ERC20Transfer]` | All ERC-20 `Transfer` events. |
| `erc721_transfers`| `List[ERC721Transfer]`| All ERC-721 `Transfer` events. |
| `erc1155_transfers`|`List[ERC1155Transfer]`| All ERC-1155 `TransferSingle` and `TransferBatch` events. |
| `approvals` | `List[ERC20Approval]`| All ERC-20 `Approval` events. |
| `mints` | `List[MintAction]` | Liquidity addition events (e.g., Uniswap V2 Mint). |
| `burns` | `List[BurnAction]` | Liquidity removal events (e.g., Uniswap V2 Burn). |
| `deposits` | `List[DepositAction]`| Wrapper ETH `Deposit` events. |
| `withdraws` | `List[WithdrawAction]`| Wrapper ETH `Withdrawal` events. |
| `uniswap_v2_swaps` |`List[UniswapV2Swap]`| All Uniswap V2 `Swap` events. |
| `uniswap_v2_syncs`| `List[UniswapV2Sync]`| All Uniswap V2 `Sync` (reserve update) events. |
| `uniswap_v3_swaps` |`List[UniswapV3Swap]`| All Uniswap V3 `Swap` events. |
| `uniswap_v4_swaps` |`List[UniswapV4Swap]`| All Uniswap V4 `Swap` events. |
| `pair_events` | `List[PairAction]` | DEX pool `PairCreated` events. |
| `owner_events` | `List[OwnerEvent]` | `OwnershipTransferred` events. |
| `contract_creation_events`| `List[ContractCreationEvent]`| Synthetic event with info about a newly created contract. |
| `trading_enabled_events`| `List[TradingEnabledEvent]`| Events indicating that trading was enabled for a token. |
| `trading_disabled_events`|`List[TradingDisabledEvent]`| Events indicating that trading was disabled for a token. |
| `other_events` | `List[Dict]` | A list for any other recognized but uncategorized events. |

---

### **3.5 Internal Transactions (from Trace Data)**

This is a critical field derived from the transaction trace, revealing all "hidden" ETH movements between contracts that are not present in the standard event logs.

| Field | Type | Description |
| :--- | :--- | :--- |
| `internal_transactions`| `List[InternalTransaction]` | A list of all internal calls that transferred ETH (`value > 0`). Each item contains `from_address`, `to_address`, and `value`. |

---

### **3.6 State Information (Optional)**

These fields can optionally be populated to provide a snapshot of state changes.

| Field | Type | Description |
| :--- | :--- | :--- |
| `state_changes` | `Dict` | Detailed state changes (e.g., balance changes, storage diffs). |
| `latest_states` | `Dict` | The final state of relevant variables after the transaction. |

---

## 4. Future Enhancements and Uncovered Data

While the `ProcessedTransaction` model is comprehensive, there are several areas of blockchain data and analysis that are not yet explicitly covered. These represent opportunities for future enhancements to create an even more powerful analytical tool.

| Area of Enhancement | Description | Importance |
| :--- | :--- | :--- |
| **Detailed State Diffs** | The current `state_changes` is a generic dictionary. A future version could have structured fields for `balanceDiffs`, `nonceDiffs`, and detailed `storageDiffs` (contract, slot, old value, new value). | **High**: Essential for security analysis, precise state verification, and auditing complex contract interactions. |
| **Pre-State Tracing** | Analysis is currently based on the transaction's execution trace. Capturing the blockchain state *before* the transaction using a `prestateTracer` would enable deeper insights. | **High**: Crucial for MEV analysis, simulating transaction outcomes under different conditions, and security research. |
| **EIP-4844 Blob Data** | With the Dencun upgrade, transactions can carry "blobs" for L2s. The model does not yet include `blobGasUsed`, `blobGasPrice`, or `blobVersionedHashes`. | **High**: Necessary for analyzing L2 scaling solutions and their impact on L1. |
| **Token Metadata Enrichment** | While contract creation is captured, the model doesn't continuously enrich token data with information from external sources like CoinGecko, DEX Screener, or security audit platforms. | **Medium**: Would provide valuable real-time context like market prices, liquidity, and known security risks (e.g., honeypot detection). |
| **Transaction Signature Data** | The raw signature components (`v`, `r`, `s`) are not stored. Analyzing these can help identify the wallet or signing library used. | **Medium**: Useful for user behavior analysis and certain types of security forensics. |
| **EIP-2930 Access Lists** | The optional access lists that help reduce gas costs are not parsed or stored. | **Low**: Provides insights into how sophisticated users and wallets are optimizing their transactions. |
| **Contract Bytecode** | The model identifies contract addresses but does not fetch their deployed bytecode (`eth_getCode`). | **Low**: Useful for off-chain analysis, similarity comparisons, and identifying unverified contracts. |
