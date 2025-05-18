"""
Test script to directly check if the Ethereum node is accessible
and if it's returning pending transactions.
"""

import json
import sys
from web3 import Web3

def main():
    # Try to connect to the Ethereum node
    print("Attempting to connect to Ethereum node...")
    try:
        # Connect to WebSocket endpoint
        w3 = Web3(Web3.WebsocketProvider('ws://localhost:8546'))
        
        # Check if connected
        if not w3.is_connected():
            print("Failed to connect to Ethereum node")
            return False
        
        print(f"Connected to Ethereum node! Chain ID: {w3.eth.chain_id}")
        print(f"Current block number: {w3.eth.block_number}")
        
        # Try to access txpool if available (Geth specific API)
        print("\nAttempting to access transaction pool (may not work on all nodes)...")
        try:
            # This is a non-standard Ethereum JSON-RPC method available in Geth
            txpool_status = w3.provider.make_request("txpool_status", [])
            if "result" in txpool_status:
                result = txpool_status["result"]
                pending = int(result.get("pending", "0"), 16)
                queued = int(result.get("queued", "0"), 16)
                print(f"Transaction pool status: {pending} pending, {queued} queued")
            else:
                print(f"Error getting txpool status: {txpool_status}")
                
            # Try to get some pending transactions
            if pending > 0:
                print("\nTrying to fetch pending transaction details...")
                txpool_content = w3.provider.make_request("txpool_content", [])
                if "result" in txpool_content and "pending" in txpool_content["result"]:
                    pending_txs = txpool_content["result"]["pending"]
                    
                    # Sample up to 3 transactions
                    count = 0
                    for addr in pending_txs:
                        for nonce in pending_txs[addr]:
                            tx = pending_txs[addr][nonce]
                            print(f"Sample tx: From {addr}, To: {tx.get('to', 'contract_creation')}, Value: {tx.get('value', '0')}")
                            count += 1
                            if count >= 3:
                                break
                        if count >= 3:
                            break
                else:
                    print("No pending transactions found or txpool_content not supported")
        except Exception as e:
            print(f"Error accessing transaction pool: {e}")
            print("This is normal for non-Geth nodes or nodes without txpool API enabled")
        
        # Try to subscribe to new pending transactions
        print("\nAttempting to subscribe to new pending transactions...")
        try:
            # This will throw if the node doesn't support WebSocket subscriptions
            subscription_id = w3.provider.make_request(
                "eth_subscribe", ["newPendingTransactions"]
            )
            print(f"Successfully subscribed to pending transactions: {subscription_id}")
            
            # Wait for a few transactions
            print("Waiting for new pending transactions (5 second timeout)...")
            
            # Use a helper to get pending transactions
            from web3.middleware import geth_poa_middleware
            w3.middleware_onion.inject(geth_poa_middleware, layer=0)
            
            import time
            start_time = time.time()
            count = 0
            
            while time.time() - start_time < 5:
                try:
                    # This is a polling approach since we can't easily use the subscription in a script
                    latest_block = w3.eth.get_block('pending')
                    if latest_block and latest_block.transactions:
                        print(f"Found {len(latest_block.transactions)} transactions in pending block")
                        for tx_hash in latest_block.transactions[:3]:  # Show first 3
                            print(f"  Pending tx: {tx_hash.hex()}")
                        break
                    time.sleep(0.5)
                except Exception as e:
                    print(f"Error polling pending block: {e}")
                    break
            else:
                print("No pending transactions received in 5 seconds")
                
        except Exception as e:
            print(f"Error subscribing to pending transactions: {e}")
        
        return True
        
    except Exception as e:
        print(f"Failed to connect: {e}")
        return False


if __name__ == "__main__":
    sys.exit(0 if main() else 1) 