import asyncio
import time
import cProfile
import pstats
import io
from web3 import Web3

# Import your existing modules
from eth_block_processor.blockchain.block_fetcher import BlockFetcher
from eth_block_processor.alert.alert_manager import AlertManager
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.utils.logger import get_logger


logger = get_logger("profile_alert_manager")


async def profile_alerts_in_block(block_number=None):
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if block_number is None:
        block_number = w3.eth.block_number
    block_fetcher = BlockFetcher(node_url="http://127.0.0.1:8545")
    transaction_analyzer = TransactionAnalyzer(w3=w3)
    alert_manager = AlertManager()

    # Fetch the block with full transaction objects
    block = await block_fetcher.fetch_block_by_number(block_number)
    transactions = block['transactions']

    # Initialize timing data structures
    alert_timings_per_alert = {}
    total_time_start = time.time()

    # Process each transaction
    for txn in transactions:
        # Analyze the transaction
        detailed_txn = transaction_analyzer.analyze_transaction(txn)

        # Profile alert processing
        start_time = time.time()
        alerts = await alert_manager.check_alerts_async(detailed_txn)
        duration = time.time() - start_time

        # Collect alert timings per alert type
        for alert in alerts:
            alert_name = alert.alert_type
            alert_timings_per_alert.setdefault(alert_name, []).append(duration)

    total_duration = time.time() - total_time_start
    print(f"Total time to process alerts in block {block_number}: {total_duration:.6f}s")

    # Output timing information
    for alert_name, durations in alert_timings_per_alert.items():
        total_alert_time = sum(durations)
        avg_alert_time = total_alert_time / len(durations)
        print(f"Alert '{alert_name}' total time: {total_alert_time:.6f}s, average per txn: {avg_alert_time:.6f}s")

if __name__ == "__main__":
    block_number = 12345678  # Replace with the block number you want to profile
    asyncio.run(profile_alerts_in_block())
