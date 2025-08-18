#!/usr/bin/env python3
"""
Simulate Historical Transaction as Mempool Transaction

Algorithm:
1. Take a known mined transaction
2. Use debug_traceCall to simulate it against the previous block state
3. Compare results with actual transaction execution
4. Validate that our mempool simulation logic works correctly

This bridges the gap between historical analysis and mempool prediction.
"""

import asyncio
from web3 import Web3
from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
import json

async def simulate_historical_transaction():
    # Connect to local reth node
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    print(f"Connected to node: {w3.is_connected()}")
    
    # Initialize processors
    txn_data_fetcher = TransactionDataFetcher(w3)
    txn_processor = TransactionProcessor(w3, calculate_state_changes=True)
    
    # Target transaction from your validation tests
    tx_hash = "0xcbf2b9ddf1b2040c4d7f0f52aafd5ca5d21c51fc86f9002efb9a1d97698a5e29"
    mined_block = 22589865
    simulation_block = mined_block - 1  # Simulate against previous block
    
    print(f"\n🔍 Simulating Historical Transaction as Mempool Transaction:")
    print(f"   Hash: {tx_hash}")
    print(f"   Actually mined in block: {mined_block}")
    print(f"   Simulating against block: {simulation_block}")
    
    # Get the original transaction data (already mined)
    print("\n📥 Fetching historical transaction data...")
    actual_txn_data = txn_data_fetcher.get_transaction_data(tx_hash)
    
    # Process the actual historical transaction
    actual_dtxn = await txn_processor.process_transaction_async(
        actual_txn_data['transaction'], 
        actual_txn_data['receipt'], 
        actual_txn_data['trace'],
        state_diff=True
    )
    
    print("   ✅ Processed actual transaction")
    print(f"   Actual ETH transfers: {len(actual_dtxn.internal_transactions)}")
    print(f"   Actual ERC20 transfers: {len(actual_dtxn.erc20_transfers)}")
    
    # Now simulate the same transaction as if it were in mempool
    print(f"\n⚡ Simulating transaction against block {simulation_block}...")
    
    # Prepare debug_traceCall parameters
    tx = actual_txn_data['transaction']
    call_params = {
        "from": tx['from'],
        "to": tx['to'],
        "value": hex(tx['value']),
        "gas": hex(tx['gas']),
        "gasPrice": hex(tx.get('gasPrice', 0)),
        "input": tx['input']
    }
    
    # Simulate the transaction against the previous block
    try:
        simulation_trace = w3.manager.request_blocking(
            "debug_traceCall", 
            [call_params, hex(simulation_block), {"enableMemory": False, "enableReturnData": False}]
        )
        
        print("   ✅ Simulation successful!")
        print(f"   Simulated gas used: {int(simulation_trace.get('gasUsed', '0x0'), 16)}")
        print(f"   Actual gas used: {actual_txn_data['receipt']['gasUsed']}")
        
        sim_success = not simulation_trace.get('failed', False)
        actual_success = actual_txn_data['receipt']['status'] == 1
        
        print(f"   Simulated success: {sim_success}")
        print(f"   Actual success: {actual_success}")
        
        # Extract ETH transfers from simulation
        sim_eth_transfers = extract_eth_transfers_from_trace(simulation_trace)
        actual_eth_transfers = [
            (tx.from_address, tx.to_address, tx.value) 
            for tx in actual_dtxn.internal_transactions 
            if tx.value > 0
        ]
        
        print(f"\n💰 ETH Transfer Comparison:")
        print(f"   Simulated transfers: {len(sim_eth_transfers)}")
        print(f"   Actual transfers: {len(actual_eth_transfers)}")
        
        if len(sim_eth_transfers) == len(actual_eth_transfers):
            print("   ✅ Transfer count matches!")
            
            # Compare individual transfers
            matches = 0
            for i, (sim_from, sim_to, sim_value) in enumerate(sim_eth_transfers):
                if i < len(actual_eth_transfers):
                    actual_from, actual_to, actual_value = actual_eth_transfers[i]
                    if (sim_from.lower() == actual_from.lower() and 
                        sim_to.lower() == actual_to.lower() and 
                        abs(sim_value - actual_value) < 0.0001):  # Small tolerance for float precision
                        matches += 1
            
            print(f"   Transfer details match: {matches}/{len(sim_eth_transfers)}")
            
            if matches == len(sim_eth_transfers):
                print("   🎯 Perfect simulation! Would have predicted actual results.")
            else:
                print("   ⚠️  Some transfer details differ")
        else:
            print("   ❌ Transfer count mismatch")
            print(f"   Simulated: {sim_eth_transfers}")
            print(f"   Actual: {actual_eth_transfers}")
        
        # State changes comparison
        print(f"\n📊 State Changes Analysis:")
        if hasattr(actual_dtxn, 'state_changes') and actual_dtxn.state_changes:
            print(f"   Actual state changes: {len(actual_dtxn.state_changes)} addresses")
            for address, changes in actual_dtxn.state_changes.items():
                print(f"     {address}: token_net={changes.get('token_net', 0)}, eth_net={changes.get('eth_net', 0)}")
        else:
            print("   No state changes recorded in actual transaction")
            
    except Exception as e:
        print(f"   ❌ Simulation failed: {e}")
        return
    
    print(f"\n🎯 Simulation Validation Results:")
    print(f"   • Gas prediction accuracy: {'✅' if abs(int(simulation_trace.get('gasUsed', '0x0'), 16) - actual_txn_data['receipt']['gasUsed']) < 1000 else '❌'}")
    print(f"   • Success prediction: {'✅' if sim_success == actual_success else '❌'}")
    print(f"   • ETH transfers: {'✅' if len(sim_eth_transfers) == len(actual_eth_transfers) else '❌'}")
    print(f"\n   This validates that mempool simulation can predict transaction effects!")

def extract_eth_transfers_from_trace(trace):
    """Extract ETH transfers from debug_traceCall result"""
    transfers = []
    
    def process_calls(calls, depth=0):
        if not calls:
            return
            
        for call in calls:
            value_hex = call.get('value', '0x0')
            if value_hex and value_hex != '0x0':
                try:
                    value_eth = int(value_hex, 16) / 1e18
                    if value_eth > 0.0001:  # Filter out dust
                        transfers.append((
                            call.get('from', ''),
                            call.get('to', ''),
                            value_eth
                        ))
                except:
                    pass
            
            # Process nested calls
            if 'calls' in call:
                process_calls(call['calls'], depth + 1)
    
    # Process top-level calls
    if 'calls' in trace:
        process_calls(trace['calls'])
    
    return transfers

if __name__ == "__main__":
    asyncio.run(simulate_historical_transaction()) 