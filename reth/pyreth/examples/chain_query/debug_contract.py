#!/usr/bin/env python3
"""
Debug contract detection
"""

from pyreth import chain_query as pyreth_chain_query

def main():
    query = pyreth_chain_query()
    
    # Known contracts
    usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    
    # Test single contract check
    print("Single contract check:")
    is_contract = query.is_contract(usdc)
    print(f"USDC is contract: {is_contract}")
    print()
    
    # Test batch
    print("Batch contract check:")
    addresses = [
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",  # Vitalik
        usdc,  # USDC contract
    ]
    
    results = query.batch_is_contract(addresses)
    print(f"Raw results: {results}")
    print(f"Type: {type(results)}")
    
    # Print all keys
    print("\nKeys in dictionary:")
    for key in results:
        print(f"  {key}: {results[key]}")

if __name__ == "__main__":
    main()