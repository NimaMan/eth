#!/usr/bin/env python3
"""Test ZMQ alert reception from mempool processor"""

import zmq
import json
import time
from datetime import datetime

def main():
    context = zmq.Context()
    socket = context.socket(zmq.SUB)
    socket.connect("tcp://localhost:5559")
    socket.setsockopt_string(zmq.SUBSCRIBE, "")  # Subscribe to all messages
    
    print(f"[{datetime.now()}] Connected to ZMQ alert publisher on tcp://localhost:5559")
    print("Waiting for alerts...")
    
    try:
        while True:
            # Non-blocking receive with timeout
            if socket.poll(1000):  # 1 second timeout
                message = socket.recv_string()
                try:
                    alert = json.loads(message)
                    print(f"\n[{datetime.now()}] Received alert:")
                    print(f"  Alert ID: {alert.get('alert_id')}")
                    print(f"  Type: {alert.get('event_type')}")
                    print(f"  Severity: {alert.get('severity')}")
                    print(f"  TX Hash: {alert.get('tx_hash')}")
                    print(f"  Pool: {alert.get('pool_address')}")
                    print(f"  Token: {alert.get('token_address')} ({alert.get('token_symbol')})")
                    print(f"  ETH Change: {alert.get('eth_change_amount'):.6f} ETH ({alert.get('eth_change_percent'):.2f}%)")
                    print(f"  Details: {alert.get('details')}")
                except json.JSONDecodeError as e:
                    print(f"Failed to parse message: {e}")
                    print(f"Raw message: {message}")
            else:
                print(".", end="", flush=True)
                
    except KeyboardInterrupt:
        print("\nShutting down...")
    finally:
        socket.close()
        context.term()

if __name__ == "__main__":
    main()