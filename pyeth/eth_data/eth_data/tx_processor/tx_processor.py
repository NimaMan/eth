"""
Transaction Analyzer for Ethereum Blockchain

Objective:
---------
To dissect a single Ethereum transaction, along with its receipt and trace data, into a structured `ProcessedTransaction` object. This involves decoding logs, identifying key events (transfers, swaps, approvals), extracting internal ETH movements from traces, classifying the transaction type, calculating fees, and optionally computing state differences. The goal is to create a rich, standardized representation of the transaction suitable for various downstream analyses (PnL, fund flow, scam detection, etc.).

# Algorithm Overview (`process_transaction` / `process_transaction_async`)

1.  **Input**: Raw `transaction` dictionary, `receipt` dictionary, `trace` dictionary (optional), `block_timestamp`.
2.  **Basic Info Extraction**: Get hash, from/to addresses, value, nonce, input data, status, contract address (if creation) from transaction and receipt.
3.  **Fee Calculation (`_extract_transaction_fees`)**: Calculate `gas_used * effective_gas_price` from the receipt.
4.  **Log Processing (`log_processor.process_logs`)**: Decode event logs from the receipt using known ABIs/event signatures. Categorize logs into specific types (ERC20/721/1155 transfers, Uniswap events, approvals, etc.) and extract relevant data. Collect unique addresses and ERC-20 contract addresses encountered in logs.
5.  **Trace Processing (`trace_processor.process_trace`)**: If the transaction involves a contract call (`needs_trace`) and a trace is provided, parse the trace structure to identify internal ETH transfers (call/delegatecall with value > 0) and potentially other internal contract interactions. Collect addresses from internal transactions.
6.  **Address Aggregation (`extend_unique_addresses`)**: Combine addresses from the transaction (from/to), logs, internal transactions, and created contract address into a single set of unique participants.
7.  **Transaction Classification (`transaction_classifier.classify_transaction`)**: Analyze transaction input data, target address (`to`), and potentially logs/value to assign a high-level type (e.g., "Swap", "Transfer", "Contract Creation", "Approval", "Trading Enabled").
8.  **Synthetic Event Generation (`_add_tx_type_events`)**: Based on the classified `tx_type`, potentially add synthetic events to the processed logs (e.g., add `ContractCreationEvent` if type is "Contract Creation" and contract info is available).
9.  **Bribe Calculation (`_get_bribe_amount`)**: Sum the value of internal ETH transfers directed to known fee recipients/builder addresses.
10. **Action Identification (`action_identifier.identify_transaction_actions`)**: Based on the `tx_type` and decoded logs, identify higher-level actions performed by the transaction (e.g., "Swap ETH for Token", "Add Liquidity").
11. **State Change Calculation (Optional) (`_calculate_state_changes`)**: If `calculate_state_changes` is enabled, use `ProcessedTxStateDiffCalculator` (which likely needs the trace/state diff data from the node) to compute detailed state changes (balance changes, storage diffs).
12. **Assemble Output**: Create and return a `ProcessedTransaction` data model instance containing all the extracted and processed information.

Key Components and Flow:
----------------------
1. Transaction Receipt Analysis:
   - Fetches and processes transaction receipts (Assumed fetched by caller, e.g., `TransactionBatchProcessor`)
   - Extracts gas usage and effective prices (`_extract_transaction_fees`)
   - Determines transaction status and contract creation (from receipt fields)

2. Log Analysis (`TransactionLogProcessor`):
   - Processes event logs for common DeFi and token operations
   - Identifies token transfers (ERC20, ERC721, ERC1155)
   - Tracks Uniswap interactions and liquidity events
   - Maintains sets of unique addresses and contract interactions

3. Trace Analysis (Optional) (`TransactionTraceProcessor`):
   - Performed for transactions with contract interactions if trace data provided
   - Tracks internal ETH transfers and contract calls
   - Builds a list/tree of internal transactions (`InternalTransaction` objects)

4. State Difference Analysis (Optional) (`ProcessedTxStateDiffCalculator`):
   - Captures state changes in contract storage (Requires state diff data from node)
   - Tracks balance changes and storage modifications

5. Classification & Identification:
    - `EthTransactionClassifier`: Determines broad transaction type.
    - `TransactionActionIdentifier`: Determines specific actions based on type and logs.

Performance Characteristics:
-------------------------
- Primarily CPU-bound for decoding logs and processing traces once data is available.
- Some operations might involve Web3 calls (e.g., `get_erc20_contract_info` within `_add_tx_type_events`), potentially adding I/O latency if not cached.

Usage:
-----
This processor is typically invoked by a higher-level component like `TransactionBatchProcessor` which handles fetching the necessary transaction, receipt, and trace data from the Ethereum node.

Note on Design Choice:
------------------------------
The core `process_transaction` logic is synchronous, assuming the caller provides the required data (tx, receipt, trace). Asynchronous operations are handled by the caller (e.g., fetching data in batches). The `process_transaction_async` method provides an async wrapper but performs the same core synchronous logic internally after awaiting data fetching by the caller.
"""
from web3 import Web3
from typing import Any, Dict, List, Optional
from eth_data.chain_utils.common_addresses import fee_recipients
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction, TransactionFees
from eth_data.tx_processor.tx_type_classifier import EthTransactionClassifier, EthProtocolTypeClassifier
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_log_processor import TransactionLogProcessor
from eth_data.tx_processor.tx_trace_processor import TransactionTraceProcessor
from eth_data.tx_processor.address_balance_change_calculator import AddressBalanceChangeCalculator
from eth_data.tx_processor.data_models.receipt_models import TradingEnabledEvent
from eth_data.tx_processor.tx_action_identifier import TransactionActionIdentifier
from eth_data.tx_processor.data_models.trace_models import InternalTransaction
from eth_data.tx_processor.data_models.tx_models import ContractCreationEvent, ETHTransfer
        

class TransactionProcessor:
    def __init__(self, w3: Web3 = None, calculate_address_balance_changes: bool = False, eth_state_change_threshold: int = 0.005):
        self.w3 = w3
        self.calculate_address_balance_changes = calculate_address_balance_changes
        self.transaction_classifier = EthTransactionClassifier(w3=w3)
        self.protocol_classifier = EthProtocolTypeClassifier()
        self.data_fetcher = TransactionDataFetcher(w3=w3)
        self.log_processor = TransactionLogProcessor(w3=w3)
        self.trace_processor = TransactionTraceProcessor(w3=w3)
        self.address_balance_change_calculator = AddressBalanceChangeCalculator(eth_state_change_threshold=eth_state_change_threshold)
        self.action_identifier = TransactionActionIdentifier()

    def _normalize_int(self, value: Any) -> int:
        if isinstance(value, int):
            return value
        if isinstance(value, str):
            return int(value, 16) if value.startswith('0x') else int(value)
        raise TypeError(f"Unable to normalize value of type {type(value).__name__} to int")

    def _to_checksum_address(self, address: Optional[str]) -> Optional[str]:
        if not address:
            return None
        if isinstance(address, str) and self.w3.is_address(address):
            return self.w3.to_checksum_address(address)
        return None

    def needs_trace(self, tx: Dict[str, Any]) -> bool:
        # Always get trace for contract creation transactions
        if tx['to'] is None:
            return True
        # Otherwise check if there's input data
        return len(tx['input']) > 2  # '0x' is 2 characters

    def extend_unique_addresses(self, 
                                from_address, 
                                to_address, 
                                internal_transactions, 
                                unique_addresses, 
                                erc20_contracts, 
                                contract_address=None):       
        from_checksum = self._to_checksum_address(from_address)
        to_checksum = self._to_checksum_address(to_address)
        if from_checksum:
            unique_addresses.add(from_checksum)
        if to_checksum:
            unique_addresses.add(to_checksum)
        normalized_contracts = set()
        for address in erc20_contracts:
            checksum = self._to_checksum_address(address)
            if checksum:
                unique_addresses.add(checksum)
                normalized_contracts.add(checksum)
        for internal_tx in internal_transactions:
            if internal_tx.from_address:
                unique_addresses.add(internal_tx.from_address)
            if internal_tx.to_address:
                unique_addresses.add(internal_tx.to_address)
        if contract_address:
            checksum_contract = self._to_checksum_address(contract_address)
            if checksum_contract:
                unique_addresses.add(checksum_contract)
        # remove None from unique_addresses if it exists
        if None in unique_addresses:
            unique_addresses.remove(None)
        # remove WETH from erc20_contracts if it exists
        if '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' in normalized_contracts:
            normalized_contracts.remove('0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2')
        return normalized_contracts, unique_addresses
    
    def _extract_transaction_fees(self, transaction: Dict[str, Any], receipt: Dict[str, Any]) -> TransactionFees:
        """Extract transaction fee information from transaction and receipt"""
        # Get effective gas price and gas used from receipt
        effective_gas_price = self._normalize_int(receipt.get('effectiveGasPrice') or 0)
        gas_used = self._normalize_int(receipt.get('gasUsed') or 0)
        total_fee = effective_gas_price * gas_used

        gas_limit_raw = transaction.get('gas')
        if gas_limit_raw is None:
            gas_limit_raw = transaction.get('gasLimit', 0)
        gas_limit = self._normalize_int(gas_limit_raw)

        # Get gas fields - priority fee will be calculated later in ranking module
        # Protocol classifier returns snake_case keys for its calculated fields
        gas_fields = self.protocol_classifier.get_gas_fields(transaction, receipt)
        
        # Raw transaction data uses camelCase
        max_fee_per_blob_gas = transaction.get("maxFeePerBlobGas")
        if max_fee_per_blob_gas is not None:
            max_fee_per_blob_gas = self._normalize_int(max_fee_per_blob_gas)
            
        # Raw receipt data uses camelCase
        blob_gas_used = receipt.get("blobGasUsed")
        if blob_gas_used is not None:
            blob_gas_used = self._normalize_int(blob_gas_used)

        return TransactionFees(
            gas_price=effective_gas_price,
            gas_used=gas_used,
            gas_limit=gas_limit,
            tx_fee=total_fee,
            protocol_type=gas_fields.get('protocol_type', 'unknown'),
            # gas_fields uses snake_case
            max_fee_per_gas=gas_fields.get('max_fee_per_gas'),
            max_priority_fee=gas_fields.get('max_priority_fee'),
            max_fee_per_blob_gas=max_fee_per_blob_gas,
            blob_gas_used=blob_gas_used,
        )
    
    def _add_tx_type_events(self, tx_type: str, logs: Dict[str, List[Any]], transaction: Dict[str, Any], receipt: Dict[str, Any]) -> None:
        """Add synthetic events based on transaction type"""
        if tx_type == "Trading Enabled":
            token_address = self._to_checksum_address(transaction.get('to'))
            logs['trading_enabled_events'].append(
                TradingEnabledEvent(
                    token_address=token_address,
                    block_number=self._normalize_int(receipt['blockNumber']),
                    log_index=0
                )
            )
            if token_address:
                logs['erc20_contracts'].add(token_address)
        elif tx_type == "Contract Creation":
            contract_address = self._to_checksum_address(receipt.get('contractAddress'))
            if contract_address:
                 logs['contract_creation_events'].append(
                    ContractCreationEvent(contract_address=contract_address)
                )
                
    def _get_block_timestamp(self, receipt: Dict[str, Any]) -> int:
        timestamp = receipt.get('blockTimestamp')
        if timestamp is not None:
            return self._normalize_int(timestamp)
        for log in receipt.get('logs', []):
            log_timestamp = log.get('blockTimestamp')
            if log_timestamp is not None:
                return self._normalize_int(log_timestamp)
        return 0
    
    def _get_bribe_amount(self, internal_transactions: List[InternalTransaction]) -> int:
        bribe_amount = 0
        for internal_tx in internal_transactions:
            if internal_tx.to_address in fee_recipients:
                bribe_amount += internal_tx.value
        return bribe_amount

    def _extract_eth_transfers(self,
                               transaction: Dict[str, Any],
                               from_address: str,
                               to_address: str,
                               value_wei: int,
                               internal_transactions: List[InternalTransaction]) -> List:
        """
        Extract ETH transfers for simple ETH transactions.
        Only populates eth_transfers when it's a simple ETH send with no contract interaction.
        
        Args:
            transaction: The transaction dict
            from_address: Checksummed from address
            to_address: Checksummed to address  
            value: Transaction value in ETH
            internal_transactions: List of internal transactions
            
        Returns:
            List of ETHTransfer objects (empty or single item)
        """
        
        eth_transfers = []
        
        # Check if this is a simple ETH transfer:
        # 1. Has value > 0
        # 2. Has no input data (0x or empty) or minimal data
        # 3. Has no internal transactions (no contract execution)
        input_data = transaction.get('input', '0x')
        is_simple_transfer = (
            value_wei > 0 and
            (input_data == '0x' or input_data == '' or len(input_data) <= 2) and 
            len(internal_transactions) == 0 and
            to_address is not None  # Must have a recipient
        )
        
        if is_simple_transfer:
            
            eth_transfers.append(ETHTransfer(
                from_address=from_address,
                to_address=to_address,
                amount=value_wei
            ))
        
        return eth_transfers

    def _calculate_address_balance_changes(self, processed_tx: ProcessedTransaction) -> Dict[str, Any]:
        """Calculate state changes for a processed transaction"""
        if not self.calculate_address_balance_changes:
            return {}
        return self.address_balance_change_calculator.calculate_address_balance_changes_from_processed_tx(processed_tx)
    
    def process_transaction(self, 
                            transaction: Dict[str, Any], 
                            receipt: Dict[str, Any],
                            trace: Dict[str, Any],
                            block_timestamp: int = 0) -> ProcessedTransaction:
        """
        Analyzes a transaction and returns a DetailedTransaction object.
        """
        tx_hash = transaction['hash'] if isinstance(transaction['hash'], str) else transaction['hash'].hex()
        from_address = self._to_checksum_address(transaction.get('from'))
        to_address = self._to_checksum_address(transaction.get('to'))
        logs = self.log_processor.process_logs(receipt['logs'])
        fees = self._extract_transaction_fees(transaction, receipt)
        contract_address = self._to_checksum_address(receipt.get('contractAddress'))
        if block_timestamp == 0:
            block_timestamp = self._get_block_timestamp(receipt)
        
        internal_transactions = []
        if self.needs_trace(transaction) and trace:
            internal_transactions = self.trace_processor.process_trace(
                trace, 
                receipt_contract_address=contract_address 
            )
        
        unique_addresses = {self._to_checksum_address(addr) for addr in logs['unique_addresses']}
        unique_addresses.discard(None)
        erc20_contracts = {self._to_checksum_address(addr) for addr in logs['erc20_contracts']}
        erc20_contracts.discard(None)
        erc20_contracts, unique_addresses = self.extend_unique_addresses(
            from_address, 
            to_address,
            internal_transactions, 
            unique_addresses,
            erc20_contracts,
            contract_address
            )
        value_wei = self.log_processor._process_integer(transaction['value'])
        tx_type = self.transaction_classifier.classify_transaction(transaction)
        self._add_tx_type_events(tx_type, logs, transaction, receipt)
        bribe_amount = self._get_bribe_amount(internal_transactions)
        actions = self.action_identifier.identify_transaction_actions(tx_type, logs)

        raw_tx_type_value = transaction.get('type')
        raw_tx_type = self._normalize_int(raw_tx_type_value) if raw_tx_type_value is not None else 0
        
        # Extract ETH transfers for simple transfers
        eth_transfers = self._extract_eth_transfers(
            transaction, from_address, to_address, value_wei, internal_transactions
        )

        blob_hashes = transaction.get("blobVersionedHashes") or []
        normalized_blob_hashes = []
        for entry in blob_hashes:
            if isinstance(entry, bytes):
                normalized_blob_hashes.append(f"0x{entry.hex()}")
            else:
                normalized_blob_hashes.append(str(entry))

        processed_tx = ProcessedTransaction(
            hash=tx_hash,
            tx_type=tx_type,
            raw_tx_type=raw_tx_type,
            block_number=self._normalize_int(receipt['blockNumber']),
            block_timestamp=block_timestamp,
            tx_index=self._normalize_int(receipt['transactionIndex']),
            from_address=from_address,
            to_address=to_address,
            contract_address=contract_address,
            value=value_wei,
            status=bool(receipt.get('status', True)),
            nonce=self._normalize_int(transaction['nonce']),
            input=transaction['input'],
            erc20_transfers=logs['erc20_transfers'],
            erc721_transfers=logs['erc721_transfers'],
            erc1155_transfers=logs['erc1155_transfers'],
            uniswap_v2_syncs=logs['uniswap_v2_syncs'],
            uniswap_v2_swaps=logs['uniswap_v2_swaps'],
            erc20_approval_events=logs['erc20_approval_events'],
            erc721_approval_events=logs['erc721_approval_events'],
            uniswap_v2_mints=logs['uniswap_v2_mints'],
            uniswap_v2_burns=logs['uniswap_v2_burns'],
            deposit_events=logs['deposit_events'],
            withdraw_events=logs['withdraw_events'],
            contract_creation_events=logs['contract_creation_events'],
            uniswap_v2_pair_created_events=logs['uniswap_v2_pair_created_events'],
            ownership_transferred_events=logs['ownership_transferred_events'],
            trading_enabled_events=logs['trading_enabled_events'],
            trading_disabled_events=logs['trading_disabled_events'],
            other_events=logs['other_events'],
            actions=actions,
            eth_transfers=eth_transfers,
            internal_transactions=internal_transactions,
            fees=fees,
            blob_versioned_hashes=normalized_blob_hashes,
            unique_addresses=unique_addresses,
            erc20_contracts=erc20_contracts,
            bribe_amount=bribe_amount,
            uniswap_v3_pools=logs['uniswap_v3_pools'],
            uniswap_v3_initializations=logs['uniswap_v3_initializations'],
            uniswap_v3_mints=logs['uniswap_v3_mints'],
            uniswap_v3_swaps=logs['uniswap_v3_swaps'],
            uniswap_v3_positions=logs['uniswap_v3_positions'],
            uniswap_v3_increases=logs['uniswap_v3_increases'],
            uniswap_v3_decreases=logs['uniswap_v3_decreases'],
            uniswap_v4_initializes=logs['uniswap_v4_initializes'],
            uniswap_v4_modifies=logs['uniswap_v4_modifies'],
            uniswap_v4_swaps=logs['uniswap_v4_swaps'],
            permit2_events=logs['permit2_events'],
        )
        processed_tx.address_balance_changes = self._calculate_address_balance_changes(processed_tx)
        return processed_tx

    async def process_transaction_async(self, 
                                        transaction: Dict[str, Any], 
                                        receipt: Dict[str, Any] = None,
                                        trace: Dict[str, Any] = None,
                                        block_timestamp: int = 0,
                                        state_diff: bool = False) -> ProcessedTransaction:
        """Async version of process_transaction"""

        return self.process_transaction(
            transaction=transaction,
            receipt=receipt,
            trace=trace,
            block_timestamp=block_timestamp,
        )

    
