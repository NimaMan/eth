from web3 import Web3
import asyncio
import time
from typing import Dict, Any, List

from ethblockprocessor.txn.txn_analyzer import TransactionAnalyzer
from ethblockprocessor.utils.logger import get_logger


logger = get_logger()


class BlockTxnProcessor:
    """
    A class to process transactions within an Ethereum block.
    """

    def __init__(self, w3: Web3 = None, save_erc20_txn_to_db: bool = True):
        """
        Initialize the BlockTxnProcessor with a Web3 instance and TransactionAnalyzer.

        Args:
            w3 (Web3): Web3 instance connected to an Ethereum node.
            save_erc20_txn_to_db (bool): Flag to save ERC20 transactions to the database.
        """
        if w3 is None:
            w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.w3 = w3
        self.transaction_analyzer = TransactionAnalyzer(w3=self.w3, save_erc20_txn_to_db=save_erc20_txn_to_db)

    async def process_transactions(self, block: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process all transactions in the given block.

        Args:
            block (Dict[str, Any]): The block data containing transactions.

        Returns:
            Dict[str, Any]: A dictionary with transaction hashes as keys and analysis results as values.
        """
        transactions = block.get('transactions', [])
        results = {}

        start_time = time.perf_counter()

        tasks = [self.analyze_transaction(tx) for tx in transactions]
        analysis_results = await asyncio.gather(*tasks)

        end_time = time.perf_counter()
        processing_time = end_time - start_time
        logger.info(f"Processed {len(transactions)} transactions in {processing_time:.4f} seconds")

        for tx_hash, analysis in analysis_results:
            results[tx_hash] = analysis

        return results

    async def analyze_transaction(self, txn) -> (str, Any):
        """
        Analyze a single transaction.

        Args:
            txn: The transaction data.

        Returns:
            Tuple[str, Any]: The transaction hash and the analysis result.
        """
        try:
            analysis = self.transaction_analyzer.analyze_transaction(txn)
            return txn.hash.hex(), analysis
        except Exception as e:
            logger.error(f"Error processing transaction {txn.hash.hex()}: {e}")
            return txn.hash.hex(), None
