"""
Transaction Analyzer for Ethereum Blockchain

Objective:
---------
The TransactionAnalyzer serves as a comprehensive transaction parsing and analysis tool that breaks down
Ethereum transactions into their constituent components and meaningful data structures. It processes raw
transaction data into a detailed, structured format that can be used for monitoring, analysis, and alert generation.

Key Components and Flow:
----------------------
1. Transaction Receipt Analysis:
   - Fetches and processes transaction receipts
   - Extracts gas usage and effective prices
   - Determines transaction status and contract creation

2. Log Analysis:
   - Processes event logs for common DeFi and token operations
   - Identifies token transfers (ERC20, ERC721, ERC1155)
   - Tracks Uniswap interactions and liquidity events
   - Maintains sets of unique addresses and contract interactions

3. Trace Analysis (Optional):
   - Performed for transactions with contract interactions
   - Tracks internal ETH transfers and contract calls
   - Builds a tree of internal transactions

4. State Difference Analysis (Optional):
   - Captures state changes in contract storage
   - Tracks balance changes and storage modifications

Performance Characteristics:
-------------------------
- Sequential Processing: Operations are performed synchronously as each step depends on previous results
- I/O Bound: Main bottlenecks might be the RPC calls to the Ethereum node

Usage:
-----
The analyzer is typically used in two contexts:
1. Real-time monitoring of new transactions
2. Historical analysis of blockchain data

Note on Design Choice:
------------------------------
The analyzer uses synchronous Web3 calls because:
1. Operations are inherently sequential (receipt → logs → traces)
2. Each step depends on data from previous steps
3. The real performance gains come from parallel processing of multiple transactions
   rather than async processing of a single transaction's components

For parallel processing of multiple transactions, it's might be helpful to:
1. Create multiple analyzer instances
2. Process different transactions concurrently at a higher level
3. Use a transaction queue system for real-time monitoring
"""
import numpy as np
from web3 import Web3
from typing import Dict, Any, Tuple, List
from eth_block_processor.utils.common_addresses import fee_recipients
from eth_block_processor.contracts.contract_type import get_erc20_contract_info
from eth_block_processor.data_models.txn_models import ProcessedTransaction, TransactionFees
from eth_block_processor.txn.txn_type_classifier import EthTransactionClassifier
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
from eth_block_processor.txn.txn_log_processor import TransactionLogProcessor
from eth_block_processor.txn.txn_trace_processor import TransactionTraceProcessor
from eth_block_processor.txn.txn_state_diff_analyzer import TransactionStateDiffAnalyzer
from eth_block_processor.data_models.receipt_models import TradingEnabledEvent
from eth_block_processor.txn.txn_action_identifier import TransactionActionIdentifier
from eth_block_processor.data_models.trace_models import InternalTransaction
from eth_block_processor.data_models.txn_models import ContractCreationEvent


class TransactionProcessor:
    def __init__(self, w3: Web3 = None):
        self.w3 = w3
        self.transaction_classifier = EthTransactionClassifier(w3=w3)
        self.data_fetcher = TransactionDataFetcher(w3=w3)
        self.log_processor = TransactionLogProcessor(w3=w3)
        self.trace_processor = TransactionTraceProcessor(w3=w3)
        self.state_diff_analyzer = TransactionStateDiffAnalyzer(w3=w3)
        self.action_identifier = TransactionActionIdentifier()

    def needs_trace(self, txn: Dict[str, Any]) -> bool:
        return txn['to'] is not None and len(txn['input']) > 2  # '0x' is 2 characters

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
        for internal_txn in internal_transactions:
            unique_addresses.add(internal_txn.from_address)
            unique_addresses.add(internal_txn.to_address)
        if contract_address:
            unique_addresses.add(self.w3.to_checksum_address(contract_address))
        # remove None from unique_addresses if it exists
        if None in unique_addresses:
            unique_addresses.remove(None)
        # remove WETH from erc20_contracts if it exists
        if '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' in erc20_contracts:
            erc20_contracts.remove('0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2')
        return erc20_contracts, unique_addresses
    
    def _extract_transaction_fees(self, receipt: Dict[str, Any]) -> TransactionFees:
        """Extract transaction fee information from receipt"""
        # Convert hex values to integers if needed
        gas_price = int(receipt['effectiveGasPrice'], 16) if isinstance(receipt['effectiveGasPrice'], str) else receipt['effectiveGasPrice']
        gas_used = int(receipt['gasUsed'], 16) if isinstance(receipt['gasUsed'], str) else receipt['gasUsed']
        total_fee = gas_price * gas_used
        total_fee = np.float64(self.w3.from_wei(total_fee, 'ether'))
        return TransactionFees(
            gas_price=gas_price,
            gas_used=gas_used,
            txn_fee=total_fee,
        )
    
    def _add_txn_type_events(self, tx_type: str, logs: Dict[str, List[Any]], transaction: Dict[str, Any], receipt: Dict[str, Any]) -> None:
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
            contract_info = get_erc20_contract_info(contract_address, self.w3)
            if contract_info is not None:
                logs['contract_creation_events'].append(
                    ContractCreationEvent(
                        contract_address=contract_address,
                        contract_type="ERC-20",
                        symbol=contract_info['symbol'],
                        decimals=contract_info['decimals'],
                        name=contract_info['name'],
                        total_supply=contract_info['total_supply'],
                    )
                )
                
    def _get_block_timestamp(self, receipt: Dict[str, Any]) -> int:
        try:
            return int(receipt['logs'][0]['blockTimestamp'], 16) if isinstance(receipt['logs'][0]['blockTimestamp'], str) else receipt['logs'][0]['blockTimestamp']
        except Exception as e:
            return 0
    
    def _get_bribe_amount(self, internal_transactions: List[InternalTransaction]) -> float:
        bribe_amount = 0
        for internal_txn in internal_transactions:
            if internal_txn.to_address in fee_recipients:
                bribe_amount += internal_txn.value
        return bribe_amount

    def process_transaction(self, 
                            transaction: Dict[str, Any], 
                            receipt: Dict[str, Any],
                            trace: Dict[str, Any]) -> ProcessedTransaction:
        """
        Analyzes a transaction and returns a DetailedTransaction object.
        """
        txn_hash = transaction['hash'] if isinstance(transaction['hash'], str) else transaction['hash'].hex()
        from_address = self.w3.to_checksum_address(transaction['from'])
        to_address = self.w3.to_checksum_address(transaction['to']) if transaction['to'] is not None else None
        logs = self.log_processor.process_logs(receipt['logs'])
        fees = self._extract_transaction_fees(receipt)
        contract_address = receipt.get('contractAddress', None)

        internal_transactions = []
        if self.needs_trace(transaction):
            internal_transactions = self.trace_processor.process_trace(trace)
        
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
        self._add_txn_type_events(tx_type, logs, transaction, receipt)
        block_timestamp = self._get_block_timestamp(receipt)
        bribe_amount = self._get_bribe_amount(internal_transactions)
        actions = self.action_identifier.identify_transaction_actions(tx_type, logs)

        detailed_txn = ProcessedTransaction(
            hash=txn_hash,
            txn_type=tx_type,
            block_number=receipt['blockNumber'],
            block_timestamp=block_timestamp,
            txn_index=receipt['transactionIndex'],
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
            eth_transfers=[],
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
        return detailed_txn

    async def process_transaction_async(self, 
                                        transaction: Dict[str, Any], 
                                        receipt: Dict[str, Any] = None,
                                        trace: Dict[str, Any] = None,
                                        state_diff: bool = False) -> ProcessedTransaction:
        """Async version of process_transaction"""
        
        # Process logs
        logs = self.log_processor.process_logs(receipt['logs'])
        
        # Extract fees
        fees = self._extract_transaction_fees(receipt)
        
        # Get contract address if contract creation
        contract_address = receipt.get('contractAddress', None)
        
        # Process trace if needed
        internal_transactions = []
        if self.needs_trace(transaction) and trace:
            internal_transactions = self.trace_processor.process_trace(trace)
        
        # Get state diffs if requested
        if state_diff:
            state_diffs, latest_states = self.state_diff_analyzer.parse_state_diff(
                self.data_fetcher.get_state_diff(transaction['hash'])
            )
        else:
            state_diffs, latest_states = {}, {}

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
        self._add_txn_type_events(tx_type, logs, transaction, receipt)
        block_timestamp = self._get_block_timestamp(receipt)
        bribe_amount = self._get_bribe_amount(internal_transactions)
        actions = self.action_identifier.identify_transaction_actions(tx_type, logs)

        return ProcessedTransaction(
            hash=transaction['hash'],
            txn_type=tx_type,
            block_number=receipt['blockNumber'],
            block_timestamp=block_timestamp,
            txn_index=receipt['transactionIndex'],
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
            eth_transfers=logs.get('eth_transfers', []),
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
            state_diffs=state_diffs,
            latest_states=latest_states,
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

    