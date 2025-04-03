"""
Mempool Transaction Analyzer

Fetches the latest transactions from the mempool, simulates them using multiple methods,
and writes the results to JSON files for analysis.

Simulation methods used:
1. eth_call - Basic simulation that detects reverts but provides minimal data
2. trace_call with 'trace' - Records all contract calls made during transaction execution
3. trace_call with 'stateDiff' - Shows all state changes (ETH transfers and token storage changes)
4. debug_traceCall with 'callTracer' - Provides a hierarchical call tree with decoded function calls
"""

import asyncio
import json
import os
from datetime import datetime
from web3 import Web3
import logging

from eth_block_processor.txn.mempool_tx_state_diff_processor import MempoolTxProcessor

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("mempool_analyzer")

# Create Web3 instance
w3 = Web3(Web3.HTTPProvider("http://localhost:8545"))

# Output directory
OUTPUT_DIR = "/home/nima/code/crypto/logs/live"
os.makedirs(OUTPUT_DIR, exist_ok=True)

# Raw trace output directory
TRACE_OUTPUT_DIR = os.path.join(OUTPUT_DIR, "raw_traces")
os.makedirs(TRACE_OUTPUT_DIR, exist_ok=True)

async def get_mempool_transactions(max_txns=50):
    """
    Get transactions from the mempool using txpool_content API.
    
    Args:
        max_txns: Maximum number of transactions to return
        
    Returns:
        List of transaction dictionaries
    """
    try:
        # Direct JSON-RPC call to txpool_content
        payload = {
            "jsonrpc": "2.0",
            "method": "txpool_content",
            "params": [],
            "id": 1
        }
        
        logger.info("Fetching mempool transactions using txpool_content...")
        response = await asyncio.to_thread(
            w3.provider.make_request, 
            payload["method"], 
            payload["params"]
        )
        
        pending_txs = []
        
        if "result" in response and response["result"]:
            txpool_content = response["result"]
            
            # Extract pending transactions
            if 'pending' in txpool_content:
                for address in txpool_content['pending']:
                    for nonce in txpool_content['pending'][address]:
                        tx = txpool_content['pending'][address][nonce]
                        # Ensure addresses are in checksum format
                        tx['from'] = w3.to_checksum_address(tx['from'])
                        if tx.get('to'):
                            tx['to'] = w3.to_checksum_address(tx['to'])
                        pending_txs.append(tx)
                        if len(pending_txs) >= max_txns:
                            break
                    if len(pending_txs) >= max_txns:
                        break
                
                logger.info(f"Found {len(pending_txs)} pending transactions in the mempool")
            else:
                logger.warning("No 'pending' transactions found in txpool_content response")
        else:
            logger.warning("No result in txpool_content response")
            
        return pending_txs
        
    except Exception as e:
        logger.error(f"Error fetching mempool transactions: {e}")
        return []

async def simulate_transaction_with_multiple_methods(txn, w3):
    """
    Simulate a transaction using multiple methods to compare results.
    
    Args:
        txn: Transaction to simulate
        w3: Web3 instance
        
    Returns:
        Dictionary containing results from different simulation methods
    """
    # Convert transaction to simulation format
    sim_txn = {
        'from': w3.to_checksum_address(txn['from']),
        'data': txn.get('input', txn.get('data', '0x')),
        'value': int(txn.get('value', '0x0'), 16) if isinstance(txn.get('value'), str) else txn.get('value', 0),
        'gas': int(txn.get('gas', '0x0'), 16) if isinstance(txn.get('gas'), str) else txn.get('gas', 0),
        'gasPrice': int(txn.get('gasPrice', '0x0'), 16) if isinstance(txn.get('gasPrice'), str) else txn.get('gasPrice', 0)
    }
    
    # Add 'to' only if it exists (for contract creation)
    if txn.get('to'):
        sim_txn['to'] = w3.to_checksum_address(txn.get('to'))
    
    results = {}
    
    # Method 1: Basic eth_call simulation
    try:
        eth_call_result = await asyncio.to_thread(w3.eth.call, sim_txn, 'latest')
        results['eth_call'] = {
            'success': True,
            'result': eth_call_result.hex() if isinstance(eth_call_result, bytes) else eth_call_result
        }
    except Exception as e:
        results['eth_call'] = {
            'success': False,
            'error': str(e)
        }
    
    # Method 2: trace_call with optimized trace types (removing verbose vmTrace)
    trace_types = [
        ['trace'],
        ['stateDiff'],
        ['trace', 'stateDiff']
    ]
    
    for trace_type in trace_types:
        try:
            trace_params = [sim_txn, trace_type, 'latest']
            trace_result = await asyncio.to_thread(
                w3.provider.make_request,
                "trace_call",
                trace_params
            )
            results[f'trace_call_{"-".join(trace_type)}'] = trace_result
        except Exception as e:
            results[f'trace_call_{"-".join(trace_type)}'] = {
                'success': False,
                'error': str(e)
            }
    
    # Method 3: debug_traceCall with most useful tracers
    debug_tracers = [
        'callTracer',  # Shows call hierarchy
        'prestate'     # Shows state before execution
    ]
    
    for tracer in debug_tracers:
        try:
            debug_params = [
                sim_txn,
                'latest',
                {"tracer": tracer, "timeout": "5s"}
            ]
            debug_result = await asyncio.to_thread(
                w3.provider.make_request,
                "debug_traceCall",
                debug_params
            )
            results[f'debug_traceCall_{tracer}'] = debug_result
        except Exception as e:
            results[f'debug_traceCall_{tracer}'] = {
                'success': False,
                'error': str(e)
            }
    
    return results

def prepare_for_json(data):
    """
    Prepare transaction data for JSON serialization by converting non-serializable objects.
    
    Args:
        data: Transaction data
        
    Returns:
        JSON-serializable data
    """
    # For lists and dictionaries, recursively process each item
    if isinstance(data, list):
        return [prepare_for_json(item) for item in data]
    elif isinstance(data, dict):
        return {k: prepare_for_json(v) for k, v in data.items()}
    
    # Handle specific types
    if hasattr(data, 'to_dict'):
        # Use to_dict method if available (for custom model classes)
        return prepare_for_json(data.to_dict())
    elif hasattr(data, 'dict') and callable(data.dict):
        # For Pydantic models
        return prepare_for_json(data.dict())
    elif hasattr(data, '__dict__') and not isinstance(data, type):
        # For general objects, convert to dict (but not class types)
        return prepare_for_json(data.__dict__)
    
    # Handle bytes and other non-serializable types
    if isinstance(data, bytes):
        return data.hex()
    
    # Try to make other types serializable
    try:
        json.dumps(data)
        return data
    except (TypeError, OverflowError):
        return str(data)

async def process_mempool_transactions_with_raw_trace(max_txns=10):
    """
    Fetch, process, and save mempool transactions along with raw trace data.
    
    Args:
        max_txns: Maximum number of transactions to process
    """
    # Create processor
    processor = MempoolTxProcessor(w3=w3, logger=logger)
    
    # Get pending transactions
    transactions = await get_mempool_transactions(max_txns)
    
    if not transactions:
        logger.warning("No transactions found in the mempool.")
        return
    
    logger.info(f"Processing {len(transactions)} mempool transactions with raw trace output...")
    
    # Timestamp for file naming
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    # Process each transaction
    processed_txns = []
    raw_traces = []
    
    for i, tx in enumerate(transactions):
        tx_hash = tx.get('hash', f'unknown_{i}')
        if isinstance(tx_hash, bytes):
            tx_hash = tx_hash.hex()
            
        logger.info(f"Processing transaction {i+1}/{len(transactions)}: {tx_hash}")
        
        try:
            # Process using our standard processor
            result = await processor.process_transaction(tx)
            processed_txns.append(result)
            
            # Get raw trace data using multiple methods
            raw_trace_result = await simulate_transaction_with_multiple_methods(tx, w3)
            
            # Create clean entry for JSON
            raw_trace_entry = {
                'tx_hash': tx_hash,
                'from': tx.get('from'),
                'to': tx.get('to'),
                'value': tx.get('value'),
                'input': tx.get('input', '0x'),
                'trace_results': prepare_for_json(raw_trace_result)
            }
            
            raw_traces.append(raw_trace_entry)
            
        except Exception as e:
            logger.error(f"Error processing transaction: {e}")
    
    # Save processed results - we need to prepare the processed_txns for JSON
    if processed_txns:
        processed_filename = f"mempool_txns_{timestamp}.json"
        processed_filepath = os.path.join(OUTPUT_DIR, processed_filename)
        
        logger.info(f"Writing {len(processed_txns)} processed transactions to {processed_filepath}")
        
        # Apply the prepare_for_json function to processed_txns before dumping to JSON
        serializable_txns = prepare_for_json(processed_txns)
        
        with open(processed_filepath, 'w') as f:
            json.dump(serializable_txns, f, indent=2)
        
        logger.info(f"Successfully wrote processed results to {processed_filepath}")
    
    # Save raw trace results
    if raw_traces:
        raw_trace_filename = f"raw_traces_{timestamp}.json"
        raw_trace_filepath = os.path.join(TRACE_OUTPUT_DIR, raw_trace_filename)
        
        logger.info(f"Writing {len(raw_traces)} raw trace results to {raw_trace_filepath}")
        
        with open(raw_trace_filepath, 'w') as f:
            json.dump(raw_traces, f, indent=2)
        
        logger.info(f"Successfully wrote raw trace results to {raw_trace_filepath}")
        
        # Also save individual trace files for easy inspection
        for i, trace in enumerate(raw_traces):
            tx_hash = trace['tx_hash']
            individual_filename = f"trace_{timestamp}_{i}_{tx_hash[:8]}.json"
            individual_filepath = os.path.join(TRACE_OUTPUT_DIR, individual_filename)
            
            with open(individual_filepath, 'w') as f:
                json.dump(trace, f, indent=2)
                
        logger.info(f"Saved {len(raw_traces)} individual trace files for detailed inspection")
        
        # Also save sample for ERC20 transactions
        erc20_traces = [t for t in raw_traces 
                       if 'input' in t and t['input'].startswith(('0xa9059cbb', '0x23b872dd'))]
        
        if erc20_traces:
            erc20_filename = f"erc20_txs_{timestamp}.json"
            erc20_filepath = os.path.join(TRACE_OUTPUT_DIR, erc20_filename)
            
            with open(erc20_filepath, 'w') as f:
                json.dump(erc20_traces, f, indent=2)
                
            logger.info(f"Saved {len(erc20_traces)} ERC20 transaction traces for focused analysis")

if __name__ == "__main__":
    asyncio.run(process_mempool_transactions_with_raw_trace(10)) 