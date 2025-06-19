#!/usr/bin/env python3
"""Test if Reth supports newPendingTransactions with includeTransactions parameter"""

import asyncio
import json
import websockets

async def test_full_tx_subscription():
    uri = "ws://localhost:8546"
    
    async with websockets.connect(uri) as websocket:
        # Test 1: Try with includeTransactions parameter
        print("Testing subscription with includeTransactions=true...")
        subscribe_msg = {
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions", {"includeTransactions": True}],
            "id": 1
        }
        
        await websocket.send(json.dumps(subscribe_msg))
        response = await websocket.recv()
        print(f"Response: {response}")
        
        response_data = json.loads(response)
        
        if "error" in response_data:
            print("❌ Reth does not support includeTransactions parameter")
            print(f"Error: {response_data['error']}")
            
            # Test 2: Try standard subscription
            print("\nTesting standard subscription...")
            subscribe_msg = {
                "jsonrpc": "2.0",
                "method": "eth_subscribe",
                "params": ["newPendingTransactions"],
                "id": 2
            }
            
            await websocket.send(json.dumps(subscribe_msg))
            response = await websocket.recv()
            print(f"Response: {response}")
            
            # Wait for a transaction
            print("\nWaiting for first transaction...")
            notification = await websocket.recv()
            print(f"Notification: {notification[:200]}...")
            
        else:
            print("✅ Reth supports includeTransactions parameter!")
            subscription_id = response_data.get("result")
            print(f"Subscription ID: {subscription_id}")
            
            # Wait for a transaction with full data
            print("\nWaiting for first transaction with full data...")
            notification = await websocket.recv()
            notification_data = json.loads(notification)
            
            if "params" in notification_data and "result" in notification_data["params"]:
                result = notification_data["params"]["result"]
                if isinstance(result, dict) and "hash" in result:
                    print("✅ Received full transaction data!")
                    print(f"Transaction hash: {result['hash']}")
                    print(f"From: {result.get('from', 'N/A')}")
                    print(f"To: {result.get('to', 'N/A')}")
                    print(f"Value: {result.get('value', 'N/A')}")
                else:
                    print("❌ Received only transaction hash")
                    print(f"Result: {result}")

if __name__ == "__main__":
    asyncio.run(test_full_tx_subscription())