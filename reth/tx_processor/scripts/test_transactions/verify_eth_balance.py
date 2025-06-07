#!/usr/bin/env python3
"""Verify ETH movements balance correctly"""

# From the Rust output
eth_changes = {
    "0x000000000004444c5dc75cb358380d2e3de08a90": 16.325394984036183,  # V4 Pool
    "0x11b815efb8f581194ae79006d24e0d814b7697f6": 6.7296144617885,     # V3 Pool  
    "0x3177f690119f6677298118864e2bff499fa1c359": 5.495899762937539,   # Intermediate 1
    "0x5b43453fce04b92e190f391a83136bfbecedefd1": -2.2646153e-11,      # Sender
    "0x6bdf35354898802b3b9b202a6de6eae8f9d59e9d": 10.829495221098648,  # Intermediate 2
    "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2": -16.325394984036183, # WETH
    "0xfbd4cdb413e45a52e2c8312f670e9ce67e794c37": -23.05500944580204,  # Router
}

print("ETH Balance Verification")
print("=" * 50)

total = 0
for addr, change in eth_changes.items():
    print(f"{addr[:10]}...: {change:>20.15f} ETH")
    total += change

print("-" * 50)
print(f"Total: {total:.15f} ETH")
print(f"Should be close to 0 (excluding gas fees)")

print("\n\nAnalysis of flows:")
print("-" * 50)

# What should happen based on Etherscan data:
print("Expected flows based on Etherscan:")
print("1. Router sends out WETH to intermediates:")
print(f"   - To Intermediate 1: 10.829495221098646603 WETH")
print(f"   - To Intermediate 2:  5.495899762937538401 WETH")
print(f"   - To V3 Pool:         6.729614461788500138 WETH")
print(f"   Total sent: {10.829495221098646603 + 5.495899762937538401 + 6.729614461788500138} WETH")

print("\n2. WETH unwraps (WETH contract sends ETH):")
print(f"   - To Intermediate 1: 10.829495221098646603 ETH")
print(f"   - To Intermediate 2:  5.495899762937538401 ETH")
print(f"   Total unwrapped: {10.829495221098646603 + 5.495899762937538401} ETH")

print("\n3. Intermediates send ETH to V4 Pool:")
print(f"   - From Intermediate 1: 10.829495221098646603 ETH")
print(f"   - From Intermediate 2:  5.495899762937538401 ETH")
print(f"   Total to V4: {10.829495221098646603 + 5.495899762937538401} ETH")

print("\n\nISSUE IDENTIFIED:")
print("-" * 50)
print("The intermediate addresses should show NET ZERO ETH!")
print("They receive WETH (treated as ETH) and send actual ETH")
print("Current Rust shows them with positive balance - this is WRONG")
print("\nThe issue is that we're not properly accounting for the ETH they SEND OUT")