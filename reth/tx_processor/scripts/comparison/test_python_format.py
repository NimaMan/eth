#!/usr/bin/env python3

import requests
import json

# Test transaction
tx_hash = "0xc2ee34725dd0db8df65144fa70252e4a25db891e7597b4144fa3e0599174bce8"

# Make request
response = requests.post(
    f"http://127.0.0.1:18000/validate/transaction/{tx_hash}",
    json={
        "tx_hash": tx_hash,
        "include_state_changes": True
    }
)

data = response.json()

if data.get("processed_transaction"):
    state_changes = data["processed_transaction"]["state_changes"]
    
    print("Python state changes (raw):")
    for address, changes in state_changes.items():
        print(f"\nAddress: {address}")
        print(f"  eth_net: {changes['eth_net']} (type: {type(changes['eth_net'])})")
        print(f"  token_net: {changes['token_net']}")
        
        # Print raw JSON
        print(f"  Raw JSON: {json.dumps(changes, separators=(',', ':'))}")