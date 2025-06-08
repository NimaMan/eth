#!/usr/bin/env python3
import zmq
import json
import time

context = zmq.Context()
subscriber = context.socket(zmq.SUB)
subscriber.connect("tcp://localhost:5557")
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
subscriber.setsockopt(zmq.RCVTIMEO, 2000)  # 2 second timeout

print("Testing ZMQ subscriber on port 5557...")

try:
    for i in range(3):
        try:
            message = subscriber.recv_json()
            print(f"✅ Received message {i+1}:")
            print(f"   Type: {message.get('type', 'unknown')}")
            if 'data' in message:
                print(f"   Pool count: {len(message['data'])}")
                # Show first pool
                if message['data']:
                    first_pool = list(message['data'].items())[0]
                    print(f"   Sample pool: {first_pool[0]}")
                    print(f"   ETH reserve: {first_pool[1].get('eth_reserve', 0):.4f}")
        except zmq.Again:
            print(f"❌ Timeout waiting for message {i+1}")
            
except KeyboardInterrupt:
    print("\nInterrupted")
finally:
    subscriber.close()
    context.term()
    print("Done")