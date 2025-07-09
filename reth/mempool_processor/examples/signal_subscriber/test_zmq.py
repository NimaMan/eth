#!/usr/bin/env python3
"""Simple ZMQ test to verify connectivity"""

import zmq
import time

print("Testing ZMQ connection to tcp://127.0.0.1:5556...")

context = zmq.Context()
subscriber = context.socket(zmq.SUB)

try:
    subscriber.connect("tcp://127.0.0.1:5556")
    subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
    subscriber.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
    
    print("Connected! Waiting for signals (5 second timeout)...")
    
    try:
        message = subscriber.recv_string()
        print(f"✅ SUCCESS! Received signal: {message[:100]}...")
    except zmq.Again:
        print("❌ No signals received within 5 seconds")
        print("   - Make sure the signal detector is running")
        print("   - Check if any liquidity removals or trading enabled functions are being detected")
        
except Exception as e:
    print(f"❌ Error: {e}")
finally:
    subscriber.close()
    context.term()
    print("Test complete")