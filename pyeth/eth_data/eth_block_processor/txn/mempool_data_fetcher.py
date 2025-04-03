
import asyncio
from typing import Dict, List, Any, Optional, Tuple
from web3 import Web3


class MempoolDataFetcher:
    """
    Fetches transactions from the Ethereum mempool using Reth-compatible methods.
    Optimized for txpool_content API which works well with Reth nodes.
    """
    
    def __init__(self, w3: Web3 = None, logger=None):
        """
        Initialize the mempool fetcher.
        
        Args:
            w3: Web3 instance connected to an Ethereum node
            logger: Optional logger for debugging
        """
        self.w3 = w3 or Web3(Web3.HTTPProvider("http://localhost:8545"))
        self.logger = logger
    
    async def get_mempool_transactions(self) -> Tuple[Dict[str, Any], Dict[str, Any]]:
        """
        Get transactions from the mempool using txpool_content API.

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
            response = await asyncio.to_thread(
                self.w3.provider.make_request, 
                payload["method"], 
                payload["params"]
            )
            
            pending_txs = []
            queued_txs = []
            if "result" in response and response["result"]:
                txpool_content = response["result"]
                pending_txs_dict = txpool_content["pending"]
                # Extract pending transactions
                if pending_txs_dict:
                    for address in pending_txs_dict:
                        for nonce in pending_txs_dict[address]:
                            tx = pending_txs_dict[address][nonce]
                            pending_txs.append(tx)
                queued_txs_dict = txpool_content["queued"]
                if queued_txs_dict:
                    for address in queued_txs_dict:
                        for nonce in queued_txs_dict[address]:
                            tx = queued_txs_dict[address][nonce]
                            queued_txs.append(tx)
                
                return pending_txs, queued_txs
                        
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error fetching mempool transactions: {e}")
            return [], []