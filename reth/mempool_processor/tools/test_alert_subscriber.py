#!/usr/bin/env python3
"""
Test subscriber for mempool scam detection alerts
Connects to ZMQ publisher on port 5559 and displays alerts
"""

import zmq
import json
from datetime import datetime

def main():
    print("📡 Mempool Alert Subscriber starting...")
    
    # Create ZMQ context and subscriber socket
    context = zmq.Context()
    subscriber = context.socket(zmq.SUB)
    
    # Connect to the publisher
    endpoint = "tcp://localhost:5559"
    subscriber.connect(endpoint)
    
    # Subscribe to all messages (empty string = all topics)
    subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
    
    print(f"✅ Connected to {endpoint}")
    print("⏳ Waiting for alerts...\n")
    
    alert_count = 0
    
    try:
        while True:
            # Receive message (blocking)
            message = subscriber.recv_string()
            alert_count += 1
            
            # Parse JSON
            try:
                alert = json.loads(message)
                
                # Display alert
                timestamp = datetime.fromtimestamp(alert['timestamp'] / 1000)
                print(f"{'='*60}")
                print(f"🚨 ALERT #{alert_count} - {alert['event_type']} - {alert['severity']}")
                print(f"⏰ Time: {timestamp}")
                print(f"📄 TX: {alert['tx_hash']}")
                print(f"🏊 Pool: {alert['pool_address']}")
                print(f"🪙 Token: {alert['token_symbol']} ({alert['token_address']})")
                print(f"💰 ETH Change: {alert['eth_change_amount']:.4f} ETH ({alert['eth_change_percent']:.1f}%)")
                print(f"📊 Reserves: {alert['current_eth_reserve']:.4f} → {alert['simulated_eth_reserve']:.4f} ETH")
                print(f"📈 Price Impact: {alert['price_impact_percent']:.2f}%")
                print(f"🎯 Confidence: {alert['confidence_score']:.2f}")
                print(f"📝 Details: {alert['details']}")
                print(f"{'='*60}\n")
                
            except json.JSONDecodeError as e:
                print(f"❌ Failed to parse alert: {e}")
                print(f"   Raw message: {message}\n")
                
    except KeyboardInterrupt:
        print(f"\n👋 Shutting down... Received {alert_count} alerts total.")
    finally:
        subscriber.close()
        context.term()

if __name__ == "__main__":
    main()