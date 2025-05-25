"""
Mempool Processed Transaction Analyzer

Fetches mempool transactions using MempoolFetcher, processes them into 
standard ProcessedTransaction format, and saves them to a JSON file for analysis.

Includes enhanced error handling for RPC errors.
"""

import asyncio
import json
import os
import traceback
from datetime import datetime
from web3 import Web3
from typing import List, Dict, Any, Set
from dataclasses import asdict

from eth_block_processor.txn.mempool.mempool_tx_state_diff_processor import MempoolTxProcessor
from eth_block_processor.txn.mempool.mempool_data_fetcher import MempoolDataFetcher
from eth_block_processor.data_models.txn_models import ProcessedTransaction
from eth_block_processor.utils.logger import get_logger


logger = get_logger("mempool_processor")


# Create Web3 instance - add timeout for better error handling
w3 = Web3(Web3.HTTPProvider("http://localhost:8545", request_kwargs={'timeout': 60}))

# Test connection to node
try:
    is_connected = w3.is_connected()
    current_block = w3.eth.block_number
    logger.info(f"Connected to Ethereum node: {is_connected}, Current block: {current_block}")
    
    # Check if txpool API is available
    try:
        client_version = w3.client_version
        logger.info(f"Node client: {client_version}")
        
        # Test txpool API directly
        response = w3.provider.make_request("txpool_status", [])
        if "result" in response:
            logger.info(f"txpool API is available: {response['result']}")
        else:
            logger.warning(f"txpool API response format unexpected: {response}")
    except Exception as e:
        logger.error(f"Error checking txpool API: {e}")
except Exception as e:
    logger.error(f"Failed to connect to Ethereum node: {e}")

# Output directory
OUTPUT_DIR = "/home/nima/code/crypto/logs/live"
os.makedirs(OUTPUT_DIR, exist_ok=True)

def process_transaction_for_json(tx: ProcessedTransaction) -> Dict[str, Any]:
    return asdict(tx)

async def get_mempool_txs_direct(max_txns=50):
    """Direct implementation for testing when MempoolFetcher fails"""
    try:
        # Direct JSON-RPC call to txpool_content
        payload = {
            "jsonrpc": "2.0",
            "method": "txpool_content",
            "params": [],
            "id": 1
        }
        
        logger.info("Directly fetching mempool transactions using txpool_content...")
        response = await asyncio.to_thread(
            w3.provider.make_request, 
            payload["method"], 
            payload["params"]
        )
        
        # Removed debug printing of response
        
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
            error_msg = "No result in txpool_content response" 
            if "error" in response:
                error_msg += f": {response['error']}"
            logger.warning(error_msg)
            
        return pending_txs
        
    except Exception as e:
        logger.error(f"Error fetching mempool transactions: {str(e)}")
        logger.error(traceback.format_exc())
        return []

async def process_mempool_transactions(max_txns=10):  # Reduced to 10 for testing
    """
    Fetch, process, and save mempool transactions in standard format.
    
    Args:
        max_txns: Maximum number of transactions to process
    """
    try:
        # Create processor
        processor = MempoolTxProcessor(w3=w3, logger=logger)
        
        # Try to get transactions with MempoolFetcher first
        try:
            fetcher = MempoolDataFetcher(w3=w3, logger=logger)
            logger.info(f"Fetching up to {max_txns} transactions from the mempool using MempoolFetcher...")
            transactions = await fetcher.get_normalized_pending_transactions(max_txns)
        except Exception as e:
            logger.error(f"MempoolFetcher failed: {e}")
            logger.error(traceback.format_exc())
            # Fall back to direct implementation
            transactions = await get_mempool_txs_direct(max_txns)
        
        if not transactions:
            logger.warning("No transactions found in the mempool.")
            return
        
        logger.info(f"Processing {len(transactions)} mempool transactions into standard format...")
        
        # Don't log raw transactions
        
        # Timestamp for file naming
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        
        # Process each transaction
        processed_txns = []
        successful_txns = []
        failed_txns = []
        
        for i, tx in enumerate(transactions):
            tx_hash = tx.get('hash', f'unknown_{i}')
            if isinstance(tx_hash, bytes):
                tx_hash = tx_hash.hex()
                
            logger.info(f"Processing transaction {i+1}/{len(transactions)}: {tx_hash}")
            
            try:
                # Process using our updated processor that returns ProcessedTransaction objects
                result = await processor.process_transaction(tx)
                processed_txns.append(result)
                
                # Categorize based on simulation success
                if result.status == "PENDING":
                    successful_txns.append(tx_hash)
                else:
                    failed_txns.append(tx_hash)
                
            except Exception as e:
                logger.error(f"Error processing transaction {tx_hash}: {str(e)}")
                logger.error(traceback.format_exc())
                failed_txns.append(tx_hash)
        
        # Save processed results
        if processed_txns:
            processed_filename = f"processed_mempool_txns_{timestamp}.json"
            processed_filepath = os.path.join(OUTPUT_DIR, processed_filename)
            
            logger.info(f"Writing {len(processed_txns)} processed transactions to {processed_filepath}")
            
            # Convert each ProcessedTransaction to a serializable dict
            serializable_txns = [process_transaction_for_json(tx) for tx in processed_txns]
            
            with open(processed_filepath, 'w') as f:
                json.dump(serializable_txns, f, indent=2)
            
            logger.info(f"Successfully wrote results to {processed_filepath}")
            
            # Print summary statistics
            eth_transfers = sum(1 for tx in processed_txns if tx.eth_transfers)
            erc20_transfers = sum(1 for tx in processed_txns if tx.erc20_transfers)
            erc721_transfers = sum(1 for tx in processed_txns if tx.erc721_transfers)
            
            logger.info(f"Summary:")
            logger.info(f"- Total transactions: {len(processed_txns)}")
            logger.info(f"- Successfully simulated: {len(successful_txns)}")
            logger.info(f"- Failed simulation: {len(failed_txns)}")
            logger.info(f"- Transactions with ETH transfers: {eth_transfers}")
            logger.info(f"- Transactions with ERC20 transfers: {erc20_transfers}")
            logger.info(f"- Transactions with ERC721 transfers: {erc721_transfers}")
    
    except Exception as e:
        logger.error(f"Error in process_mempool_transactions: {str(e)}")
        logger.error(traceback.format_exc())

if __name__ == "__main__":
    asyncio.run(process_mempool_transactions(20)) 