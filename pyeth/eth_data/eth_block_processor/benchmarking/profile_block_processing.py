import asyncio
import time
from web3 import Web3

from eth_block_processor.blockchain.block_fetcher import BlockFetcher
from eth_block_processor.blockchain.block_processor import *
from eth_block_processor.alert.alert_manager import AlertManager, ALERT_CLASSES
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.utils.logger import get_logger

logger = get_logger("profile_block_processing")

async def profile_block_processing(block_number=None):
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if block_number is None:
        block_number = w3.eth.block_number

    node_url = "http://127.0.0.1:8545"

    # Initialize components
    block_fetcher = BlockFetcher(node_url=node_url)
    transaction_analyzer = TransactionAnalyzer(w3=w3)
    alert_manager = AlertManager()

    total_start_time = time.time()

    # Measure time to fetch the block
    start_time = time.time()
    block = await block_fetcher.fetch_block_by_number(block_number)
    time_fetch_block = time.time() - start_time
    print(f"Time to fetch block {block_number}: {time_fetch_block:.6f}s")

    transactions = block['transactions']
    txn_count = len(transactions)
    print(f"Number of transactions in block {block_number}: {txn_count}")

    # Initialize timing data structures
    times_analyze_transactions = []
    times_alerts = []
    times_per_alert = {}
    times_per_component = {}

    # Process each transaction
    for txn in transactions:
        txn_hash = txn.hash.hex()

        # Time transaction analysis
        start_time = time.time()
        detailed_txn, analyze_timings = analyze_transaction_with_timing(transaction_analyzer, txn)
        time_analyze_txn = time.time() - start_time
        times_analyze_transactions.append(time_analyze_txn)

        # Collect per-component times
        for component, duration in analyze_timings.items():
            times_per_component.setdefault(component, []).append(duration)

        # Time alert checking
        start_time = time.time()
        alerts, alert_times = await check_alerts_with_timing(alert_manager, detailed_txn)
        time_alerts = time.time() - start_time
        times_alerts.append(time_alerts)

        # Collect per-alert times
        for alert_name, duration in alert_times.items():
            times_per_alert.setdefault(alert_name, []).append(duration)

    total_time = time.time() - total_start_time
    print(f"Total time to process block {block_number}: {total_time:.6f}s\n")

    # Compute average times
    average_analyze_time = sum(times_analyze_transactions) / txn_count
    average_alert_time = sum(times_alerts) / txn_count

    print(f"Average time to analyze a transaction: {average_analyze_time:.6f}s")
    print(f"Average time to check alerts for a transaction: {average_alert_time:.6f}s\n")

    # Output per-component analysis times
    print("Average times per transaction analysis component:")
    for component, durations in times_per_component.items():
        avg_time = sum(durations) / txn_count
        print(f"  {component}: {avg_time:.6f}s")

    # Output per-alert times
    print("\nAverage times per alert:")
    for alert_name, durations in times_per_alert.items():
        avg_time = sum(durations) / txn_count
        print(f"  {alert_name}: {avg_time:.6f}s")

def analyze_transaction_with_timing(transaction_analyzer, txn):
    timings = {}
    start_time_total = time.time()

    # Fetch transaction receipt
    start_time = time.time()
    receipt = transaction_analyzer.data_fetcher.get_transaction_receipt(txn.hash)
    timings['fetch_receipt'] = time.time() - start_time

    # Analyze logs
    start_time = time.time()
    logs = transaction_analyzer.log_analyzer.analyze_logs(receipt.logs)
    timings['analyze_logs'] = time.time() - start_time

    # Compute fees
    start_time = time.time()
    fees = TransactionFees(
        gas_price=receipt['effectiveGasPrice'],
        gas_used=receipt['gasUsed'],
        total_fee=receipt['effectiveGasPrice'] * receipt['gasUsed'],
    )
    timings['compute_fees'] = time.time() - start_time

    # Trace analysis
    start_time = time.time()
    if transaction_analyzer.needs_trace(txn):
        trace = transaction_analyzer.data_fetcher.get_transaction_trace(txn.hash)
        internal_transactions = transaction_analyzer.trace_analyzer.process_trace(trace)
    else:
        internal_transactions = []
    timings['trace_analysis'] = time.time() - start_time

    # Classify transaction type
    start_time = time.time()
    tx_type = transaction_analyzer.transaction_classifier.classify_transaction(txn)
    timings['classify_txn'] = time.time() - start_time

    # Extend unique addresses
    start_time = time.time()
    unique_addresses = logs['unique_addresses']
    erc20_contracts = logs['erc20_contracts']
    erc20_contracts, unique_addresses = transaction_analyzer.extend_unique_addresses(
        txn['from'],
        txn['to'],
        internal_transactions,
        unique_addresses,
        erc20_contracts,
    )
    timings['extend_unique_addresses'] = time.time() - start_time

    # Build DetailedTransaction object
    start_time = time.time()
    detailed_txn = DetailedTransaction(
        hash=txn.hash,
        txn_type=tx_type,
        block_number=receipt['blockNumber'],
        txn_index=receipt['transactionIndex'],
        from_address=txn['from'],
        to_address=txn['to'],
        contract_address=receipt.get('contractAddress', None),
        value=txn['value'],
        status=receipt['status'],
        nonce=txn['nonce'],
        input=txn['input'],
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
        state_diffs={},
        latest_states={},
    )
    timings['build_detailed_txn'] = time.time() - start_time

    # Total analysis time
    timings['total_analysis_time'] = time.time() - start_time_total

    return detailed_txn, timings

async def check_alerts_with_timing(alert_manager, detailed_txn):
    loop = asyncio.get_event_loop()
    alert_futures = {}
    alert_times = {}

    # Schedule alerts
    for alert_name in ALERT_CLASSES.keys():
        # Start timing for individual alert
        start_time = time.time()
        # Execute alert synchronously for timing
        alert = ALERT_CLASSES[alert_name]()
        result = alert.get_alert(detailed_txn)
        duration = time.time() - start_time
        alert_times[alert_name] = duration
        alert_futures[alert_name] = result

    # Collect alerts
    alerts = []
    for result in alert_futures.values():
        if result:
            alerts.extend(result)

    return alerts, alert_times

if __name__ == "__main__":
    asyncio.run(profile_block_processing())
