from web3 import Web3
from functools import cached_property
from typing import Dict, Any, Tuple
from web3.types import TxData
from ethblockprocessor.data_models.txn_models import DetailedTransaction, TransactionFees
from ethblockprocessor.txn.txn_type_classifier import EthTransactionClassifier
from ethblockprocessor.txn.txn_data_fetcher import TransactionDataFetcher
from ethblockprocessor.txn.txn_log_analyzer import TransactionLogAnalyzer
from ethblockprocessor.txn.txn_trace_analyzer import TransactionTraceAnalyzer
from ethblockprocessor.txn.txn_state_diff_analyzer import TransactionStateDiffAnalyzer
from ethblockprocessor.tokens.erc20_token_txn_store import ERC20TransactionDB


class TransactionAnalyzer:
    def __init__(self, w3: Web3, save_erc20_txn_to_db: bool = True):
        self.w3 = w3
        self.transaction_classifier = EthTransactionClassifier(w3=w3)
        self.data_fetcher = TransactionDataFetcher(w3=w3)
        self.log_analyzer = TransactionLogAnalyzer(w3=w3)
        self.trace_analyzer = TransactionTraceAnalyzer(w3=w3)
        self.state_diff_analyzer = TransactionStateDiffAnalyzer(w3=w3)
        self.save_erc20_txn_to_db = save_erc20_txn_to_db

    def analyze_transaction(self, transaction: Dict[str, Any], state_diff: bool = False) -> DetailedTransaction:
        """
        Analyzes a transaction and returns a DetailedTransaction object.

        Args:
            transaction (TxData): The transaction data from Web3.

        Returns:
            DetailedTransaction: A comprehensive representation of the transaction.
        """
        txn_hash = transaction.hash
        from_address = transaction['from']
        to_address = transaction['to']
        receipt = self.data_fetcher.get_transaction_receipt(txn_hash)
        logs = self.log_analyzer.analyze_logs(receipt.logs)
        
        fees = TransactionFees(
            gas_price=receipt['effectiveGasPrice'],
            gas_used=receipt['gasUsed'],
            total_fee=receipt['effectiveGasPrice'] * receipt['gasUsed'],
        )
        contract_address = receipt.get('contractAddress', None)
        if state_diff:
            raw_state_diff = self.data_fetcher.get_state_diff(txn_hash)
            state_diffs, latest_states = self.state_diff_analyzer.parse_state_diff(raw_state_diff)
        else:
            state_diffs, latest_states = {}, {}

        if self.needs_trace(transaction):
            trace = self.data_fetcher.get_transaction_trace(txn_hash)
            internal_transactions = self.trace_analyzer.process_trace(trace)
        else:
            internal_transactions = []
        
        tx_type = self.transaction_classifier.classify_transaction(transaction)
        unique_addresses = logs['unique_addresses']
        erc20_contracts = logs['erc20_contracts']
        erc20_contracts, unique_addresses = self.extend_unique_addresses(
            from_address, 
            to_address,
            internal_transactions, 
            unique_addresses,
            erc20_contracts,
            )
        detailed_txn = DetailedTransaction(
            hash=txn_hash,
            txn_type=tx_type,
            block_number=receipt['blockNumber'],
            txn_index=receipt['transactionIndex'],
            from_address=from_address,
            to_address=to_address,
            contract_address=contract_address,
            value=transaction['value'],
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
            pair_events=logs['pair_events'],
            owner_events=logs['owner_events'],
            trading_enabled_events=logs['trading_enabled_events'],
            trading_disabled_events=logs['trading_disabled_events'],
            other_events=logs['other_events'],
            actions=[],
            eth_transfers=[],
            contract_interactions=[],
            internal_transactions=internal_transactions,
            fees=fees,
            unique_addresses=unique_addresses,
            erc20_contracts=erc20_contracts,
            state_diffs=state_diffs,
            latest_states=latest_states,
        )
        if self.save_erc20_txn_to_db:
            self.store_erc20_transaction(detailed_txn)
        return detailed_txn

    def needs_trace(self, txn: Dict[str, Any]) -> bool:
        return txn['to'] is not None and len(txn['input']) > 2  # '0x' is 2 characters

    def extend_unique_addresses(self, 
                                from_address, 
                                to_address, 
                                internal_transactions, 
                                unique_addresses, 
                                erc20_contracts):       
        unique_addresses.add(from_address)
        unique_addresses.add(to_address)
        for internal_txn in internal_transactions:
            unique_addresses.add(internal_txn.from_address)
            unique_addresses.add(internal_txn.to_address)
        return erc20_contracts, unique_addresses
    
    def store_erc20_transaction(self, detailed_txn: DetailedTransaction):
        if detailed_txn.txn_type == 'ERC20_TRANSFER' or\
            len(detailed_txn.erc20_contracts) > 0 or \
            len(detailed_txn.approvals) > 0 or \
            len(detailed_txn.uniswap_v2_syncs) > 0 or \
            len(detailed_txn.uniswap_v2_swaps) > 0 or \
            len(detailed_txn.mints) > 0 or \
            len(detailed_txn.burns) > 0 or \
            len(detailed_txn.deposits) > 0 or \
            len(detailed_txn.withdraws) > 0:
            with ERC20TransactionDB() as erc20_transaction_db:
                erc20_transaction_db.add_transaction(detailed_txn)
