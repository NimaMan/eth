#!/usr/bin/env python3
"""
Live pool publisher that subscribes to the actual Python pool updates.
This connects to the real pool update system if it's running.
"""

import zmq
import json
import time
import sys

def subscribe_and_republish(duration_seconds=70):
    """Subscribe to real pool updates and republish them for testing."""
    
    context = zmq.Context()
    
    # Subscribe to the real pool updates (if available)
    subscriber = context.socket(zmq.SUB)
    subscriber.connect("tcp://localhost:5556")  # Real pool publisher
    subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
    
    # Set up publisher for test
    publisher = context.socket(zmq.PUB)
    publisher.bind("tcp://*:5557")
    
    print("🚀 Live pool publisher bridge started")
    print("📡 Subscribing to real updates on tcp://localhost:5556")
    print("📤 Publishing to test on tcp://localhost:5557")
    print("-" * 60)
    
    start_time = time.time()
    message_count = 0
    
    # Set a timeout so we don't block forever
    subscriber.setsockopt(zmq.RCVTIMEO, 1000)  # 1 second timeout
    
    try:
        while time.time() - start_time < duration_seconds:
            try:
                # Try to receive from real publisher
                message = subscriber.recv_json()
                
                # Forward the message
                publisher.send_json(message)
                message_count += 1
                
                if message_count % 10 == 0:
                    print(f"📤 Forwarded {message_count} messages")
                    
            except zmq.Again:
                # Timeout - no message received
                # Send a heartbeat message
                heartbeat = {
                    "type": "heartbeat",
                    "timestamp": time.time(),
                    "data": {}
                }
                publisher.send_json(heartbeat)
                
            except Exception as e:
                print(f"⚠️  Error: {e}")
                
            # Small delay
            time.sleep(0.1)
            
    except KeyboardInterrupt:
        print("\n⚠️  Interrupted by user")
    finally:
        print(f"\n📊 Forwarded {message_count} pool updates")
        subscriber.close()
        publisher.close()
        context.term()

def check_live_system():
    """Check if the live pool publisher is running."""
    context = zmq.Context()
    subscriber = context.socket(zmq.SUB)
    
    try:
        # Try to connect to real pool publisher
        subscriber.connect("tcp://localhost:5556")
        subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
        subscriber.setsockopt(zmq.RCVTIMEO, 2000)  # 2 second timeout
        
        print("🔍 Checking for live pool publisher on port 5556...")
        
        try:
            message = subscriber.recv_json()
            print("✅ Live pool publisher detected!")
            print(f"   Message type: {message.get('type', 'unknown')}")
            if 'data' in message:
                print(f"   Pool count: {len(message['data'])}")
            return True
        except zmq.Again:
            print("❌ No live pool publisher detected on port 5556")
            print("   Falling back to test pool publisher")
            return False
            
    finally:
        subscriber.close()
        context.term()

if __name__ == "__main__":
    # Check if live system is available
    if check_live_system():
        print("\n🚀 Using live pool data bridge mode")
        subscribe_and_republish()
    else:
        print("\n🚀 Running standalone test publisher")
        print("   To use live data, start the Python pool monitor first")
        # Fall back to test publisher
        import pool_publisher_test
        pool_publisher_test.publish_pool_updates()