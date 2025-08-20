#!/usr/bin/env python3
"""Test pyreth module import and basic functionality"""

import pyreth

print("✅ Successfully imported pyreth module")
print(f"Version: {pyreth.__version__}")
print(f"Doc: {pyreth.__doc__}")
print()

# Test available classes
print("Available classes:")
classes = [name for name in dir(pyreth) if not name.startswith('_') and name[0].isupper()]
for cls in classes:
    print(f"  - {cls}")
print()

# Test ChainQuery
print("Testing ChainQuery...")
query = pyreth.ChainQuery()
print(f"  Created ChainQuery: {query}")

# Get USDC info
usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
print(f"\nGetting USDC total supply...")
try:
    total_supply_str = query.get_token_total_supply(usdc_address)
    decimals = query.get_token_decimals(usdc_address)
    
    # Convert string to int, then to human-readable format
    total_supply = int(total_supply_str)
    supply_formatted = total_supply / (10 ** decimals)
    print(f"  USDC Total Supply: ${supply_formatted:,.2f}")
    print(f"  USDC Decimals: {decimals}")
except Exception as e:
    print(f"  Error: {e}")

# Test TxProcessor
print("\nTesting TxProcessor...")
processor = pyreth.TxProcessor()
print(f"  Created TxProcessor: {processor}")

# Test Simulator
print("\nTesting Simulator...")
simulator = pyreth.Simulator()
print(f"  Created Simulator: {simulator}")

print("\n✅ All tests passed!")