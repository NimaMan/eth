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
import numpy as np
from web3 import Web3
from typing import Dict, Any, Tuple, List
from eth_data.chain_utils.common_addresses import fee_recipients
from eth_data.chain_utils.contract_type import get_erc20_contract_info_rpc
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction, TransactionFees
from eth_data.tx_processor.tx_type_classifier import EthTransactionClassifier, EthProtocolTypeClassifier
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_log_processor import TransactionLogProcessor
from eth_data.tx_processor.tx_trace_processor import TransactionTraceProcessor
from eth_data.tx_processor.address_balance_change_calculator import AddressBalanceChangeCalculator
from eth_data.tx_processor.data_models.receipt_models import TradingEnabledEvent
from eth_data.tx_processor.tx_action_identifier import TransactionActionIdentifier
from eth_data.tx_processor.data_models.trace_models import InternalTransaction
from eth_data.tx_processor.data_models.tx_models import ContractCreationEvent


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
        unique_addresses.add(from_address)
        unique_addresses.add(to_address)
        for address in erc20_contracts:
            unique_addresses.add(address)
        for internal_tx in internal_transactions:
            unique_addresses.add(internal_tx.from_address)
            unique_addresses.add(internal_tx.to_address)
        if contract_address:
            unique_addresses.add(self.w3.to_checksum_address(contract_address))
        # remove None from unique_addresses if it exists
        if None in unique_addresses:
            unique_addresses.remove(None)
        # remove WETH from erc20_contracts if it exists
        if '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' in erc20_contracts:
            erc20_contracts.remove('0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2')
        return erc20_contracts, unique_addresses
    
    def _extract_transaction_fees(self, transaction: Dict[str, Any], receipt: Dict[str, Any]) -> TransactionFees:
        """Extract transaction fee information from transaction and receipt"""
        # Get effective gas price and gas used from receipt
        effective_gas_price = int(receipt['effectiveGasPrice'], 16) if isinstance(receipt['effectiveGasPrice'], str) else receipt['effectiveGasPrice']
        gas_used = int(receipt['gasUsed'], 16) if isinstance(receipt['gasUsed'], str) else receipt['gasUsed']
        total_fee = effective_gas_price * gas_used
        total_fee_eth = np.float64(self.w3.from_wei(total_fee, 'ether'))
        
        # Get gas fields - priority fee will be calculated later in ranking module
        gas_fields = self.protocol_classifier.get_gas_fields(transaction, receipt)
        
        return TransactionFees(
            gas_price=effective_gas_price,  # For backward compatibility
            gas_used=gas_used,
            tx_fee=total_fee_eth,
            protocol_type=gas_fields.get('protocol_type', 'unknown'),
            max_fee_per_gas=gas_fields.get('max_fee_per_gas'),
            max_priority_fee=gas_fields.get('max_priority_fee')
        )
    
    def _add_tx_type_events(self, tx_type: str, logs: Dict[str, List[Any]], transaction: Dict[str, Any], receipt: Dict[str, Any]) -> None:
        """Add synthetic events based on transaction type"""
        if tx_type == "Trading Enabled":
            logs['trading_enabled_events'].append(
                TradingEnabledEvent(
                    token_address=transaction['to'],
                    block_number=transaction['blockNumber'],
                    log_index=0
                )
            )
            logs['erc20_contracts'].add(transaction['to'])
        elif tx_type == "Set Tax":
            # Add the contract address to erc20_contracts for Set Tax transactions
            logs['erc20_contracts'].add(transaction['to'])
        elif tx_type == "Contract Creation":
            contract_address = self.w3.to_checksum_address(receipt['contractAddress'])
            #TODO: can use the helper TokenChainDataFetcher.get_token_metadata defined in py/eth_token/eth_token/erc20_token/data/token_chain_data_fetcher.py 
            #contract_info = get_erc20_contract_info_rpc(contract_address, self.w3)
            #if contract_info is not None:
            logs['contract_creation_events'].append(
                    ContractCreationEvent(
                        contract_address=contract_address,
                        contract_type=None,
                        symbol=None, #contract_info['symbol'],
                        decimals=None, #contract_info['decimals'],
                        name=None, #contract_info['name'],
                        total_supply=None, #contract_info['total_supply'],
                    )
                )
                
    def _get_block_timestamp(self, receipt: Dict[str, Any]) -> int:
        try:
            return int(receipt['logs'][0]['blockTimestamp'], 16) if isinstance(receipt['logs'][0]['blockTimestamp'], str) else receipt['logs'][0]['blockTimestamp']
        except Exception as e:
            return 0
    
    def _get_bribe_amount(self, internal_transactions: List[InternalTransaction]) -> float:
        bribe_amount = 0
        for internal_tx in internal_transactions:
            if internal_tx.to_address in fee_recipients:
                bribe_amount += internal_tx.value
        return bribe_amount

    def _extract_eth_transfers(self, 
                               transaction: Dict[str, Any], 
                               from_address: str, 
                               to_address: str,
                               value: float,
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
        from eth_data.tx_processor.data_models.tx_models import ETHTransfer
        
        eth_transfers = []
        
        # Check if this is a simple ETH transfer:
        # 1. Has value > 0
        # 2. Has no input data (0x or empty) or minimal data
        # 3. Has no internal transactions (no contract execution)
        input_data = transaction.get('input', '0x')
        is_simple_transfer = (
            value > 0 and 
            (input_data == '0x' or input_data == '' or len(input_data) <= 2) and 
            len(internal_transactions) == 0 and
            to_address is not None  # Must have a recipient
        )
        
        if is_simple_transfer:
            
            eth_transfers.append(ETHTransfer(
                from_address=from_address,
                to_address=to_address,
                amount=self.log_processor._process_integer(transaction['value'])  # Keep in Wei
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
        from_address = self.w3.to_checksum_address(transaction['from'])
        to_address = self.w3.to_checksum_address(transaction['to']) if transaction['to'] is not None else None
        logs = self.log_processor.process_logs(receipt['logs'])
        fees = self._extract_transaction_fees(transaction, receipt)
        contract_address = receipt.get('contractAddress', None)
        if block_timestamp == 0:
            block_timestamp = self._get_block_timestamp(receipt)
        
        internal_transactions = []
        if self.needs_trace(transaction) and trace:
            internal_transactions = self.trace_processor.process_trace(
                trace, 
                receipt_contract_address=contract_address 
            )
        
        unique_addresses = logs['unique_addresses']
        erc20_contracts = logs['erc20_contracts']
        erc20_contracts, unique_addresses = self.extend_unique_addresses(
            from_address, 
            to_address,
            internal_transactions, 
            unique_addresses,
            erc20_contracts,
            contract_address
            )
        value = np.float64(self.w3.from_wei(self.log_processor._process_integer(transaction['value']), 'ether'))
        tx_type = self.transaction_classifier.classify_transaction(transaction)
        self._add_tx_type_events(tx_type, logs, transaction, receipt)
        bribe_amount = self._get_bribe_amount(internal_transactions)
        actions = self.action_identifier.identify_transaction_actions(tx_type, logs)
        
        # Extract ETH transfers for simple transfers
        eth_transfers = self._extract_eth_transfers(
            transaction, from_address, to_address, value, internal_transactions
        )

        processed_tx = ProcessedTransaction(
            hash=tx_hash,
            tx_type=tx_type,
            block_number=receipt['blockNumber'],
            block_timestamp=block_timestamp,
            tx_index=receipt['transactionIndex'],
            from_address=from_address,
            to_address=to_address,
            contract_address=contract_address,
            value=value,
            status=receipt['status'],
            nonce=transaction['nonce'],
            input=transaction['input'],
            erc20_transfers=logs['erc20_transfers'],
            erc721_transfers=logs['erc721_transfers'],
            erc1155_transfers=logs['erc1155_transfers'],
            uniswap_v2_syncs=logs['uniswap_v2_syncs'],
            uniswap_v2_swaps=logs['uniswap_v2_swaps'],
            approvals=logs['approvals'],
            mints=logs['mints'],
            burns=logs['burns'],
            deposits=logs['deposits'],
            withdraws=logs['withdraws'],
            contract_creation_events=logs['contract_creation_events'],
            pair_events=logs['pair_events'],
            owner_events=logs['owner_events'],
            trading_enabled_events=logs['trading_enabled_events'],
            trading_disabled_events=logs['trading_disabled_events'],
            other_events=logs['other_events'],
            actions=actions,
            eth_transfers=eth_transfers,
            internal_transactions=internal_transactions,
            fees=fees,
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
        
        # Process logs
        logs = self.log_processor.process_logs(receipt['logs'])
        
        # Extract fees
        fees = self._extract_transaction_fees(transaction, receipt)
        
        # Get contract address if contract creation
        contract_address = receipt.get('contractAddress', None)
        if block_timestamp == 0:
            block_timestamp = self._get_block_timestamp(receipt)
        
        # Process trace if needed
        internal_transactions = []
        if self.needs_trace(transaction) and trace:
            internal_transactions = self.trace_processor.process_trace(
                trace, 
                receipt_contract_address=contract_address
            )
        
        unique_addresses = logs['unique_addresses']
        from_address = transaction['from']
        to_address = transaction['to']
        erc20_contracts = logs['erc20_contracts']
        erc20_contracts, unique_addresses = self.extend_unique_addresses(
            from_address, 
            to_address,
            internal_transactions, 
            unique_addresses, 
            erc20_contracts,
            contract_address
            )
        
        value = np.float64(self.w3.from_wei(self.log_processor._process_integer(transaction['value']), 'ether'))
        tx_type = self.transaction_classifier.classify_transaction(transaction)
        self._add_tx_type_events(tx_type, logs, transaction, receipt)
        bribe_amount = self._get_bribe_amount(internal_transactions)
        actions = self.action_identifier.identify_transaction_actions(tx_type, logs)
        
        # Extract ETH transfers for simple transfers
        eth_transfers = self._extract_eth_transfers(
            transaction, from_address, to_address, value, internal_transactions
        )
        
        processed_tx = ProcessedTransaction(
            hash=transaction['hash'],
            tx_type=tx_type,
            block_number=receipt['blockNumber'],
            block_timestamp=block_timestamp,
            tx_index=receipt['transactionIndex'],
            from_address=from_address,
            to_address=to_address,
            contract_address=contract_address,
            value=value,
            status=receipt['status'],
            nonce=transaction['nonce'],
            input=transaction['input'],
            erc20_transfers=logs['erc20_transfers'],
            erc721_transfers=logs['erc721_transfers'],
            erc1155_transfers=logs['erc1155_transfers'],
            uniswap_v2_syncs=logs['uniswap_v2_syncs'],
            uniswap_v2_swaps=logs['uniswap_v2_swaps'],
            approvals=logs['approvals'],
            mints=logs['mints'],
            burns=logs['burns'],
            deposits=logs['deposits'],
            withdraws=logs['withdraws'],
            actions=actions,
            eth_transfers=eth_transfers,
            pair_events=logs.get('pair_events', []),
            owner_events=logs.get('owner_events', []),
            contract_creation_events=logs.get('contract_creation_events', []),
            trading_enabled_events=logs.get('trading_enabled_events', []),
            trading_disabled_events=logs.get('trading_disabled_events', []),
            other_events=logs.get('other_events', []),
            unique_addresses=unique_addresses,
            erc20_contracts=erc20_contracts,
            internal_transactions=internal_transactions,
            fees=fees,
            address_balance_changes={},
            latest_states={},
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

    