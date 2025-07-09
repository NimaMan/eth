#!/usr/bin/env python3
"""
ZMQ Subscriber for Mempool Signal Detector

This example demonstrates how to subscribe to real-time signals from the
mempool signal detector. It receives liquidity removal and trading enabled
signals via ZMQ.
"""

import zmq
import json
from datetime import datetime
import signal
import sys

class SignalSubscriber:
    def __init__(self, endpoint="tcp://127.0.0.1:5556"):
        self.endpoint = endpoint
        self.context = zmq.Context()
        self.subscriber = None
        self.running = True
        
        # Statistics
        self.stats = {
            'liquidity_removal': 0,
            'trading_enabled': 0,
            'total': 0
        }
        
    def connect(self):
        """Connect to the ZMQ publisher"""
        self.subscriber = self.context.socket(zmq.SUB)
        self.subscriber.connect(self.endpoint)
        self.subscriber.setsockopt_string(zmq.SUBSCRIBE, "")  # Subscribe to all messages
        
        # Set timeout to allow periodic checks
        self.subscriber.setsockopt(zmq.RCVTIMEO, 1000)  # 1 second timeout
        
        print(f"[{datetime.now()}] Connected to ZMQ publisher at {self.endpoint}")
        print("Waiting for signals...\n")
        
    def handle_signal(self, signal_data):
        """Process received signal"""
        self.stats['total'] += 1
        self.stats[signal_data['alert_type']] += 1
        
        # Display signal details
        print(f"[{datetime.now()}] SIGNAL RECEIVED! #{self.stats['total']}")
        print(f"  Type: {signal_data['alert_type'].upper()}")
        print(f"  Function: {signal_data['function_name']}")
        print(f"  TX Hash: {signal_data['tx_hash']}")
        print(f"  From: {signal_data['from_address']}")
        print(f"  To: {signal_data['to_address']}")
        print(f"  Value: {signal_data['value']}")
        print(f"  Gas Price: {signal_data['gas_price']}")
        print(f"  Selector: {signal_data['selector']}")
        print(f"  Timestamp: {signal_data['timestamp']}")
        print("-" * 80)
        
    def run(self):
        """Main subscriber loop"""
        self.connect()
        
        # Handle Ctrl+C gracefully
        signal.signal(signal.SIGINT, self._signal_handler)
        
        while self.running:
            try:
                # Try to receive a message
                message = self.subscriber.recv_string()
                
                # Parse JSON
                try:
                    signal_data = json.loads(message)
                    self.handle_signal(signal_data)
                except json.JSONDecodeError as e:
                    print(f"[ERROR] Failed to parse JSON: {e}")
                    print(f"Raw message: {message}")
                    
            except zmq.Again:
                # Timeout - no message received, continue
                continue
            except zmq.ZMQError as e:
                print(f"[ERROR] ZMQ Error: {e}")
                break
                
        self.cleanup()
        
    def _signal_handler(self, sig, frame):
        """Handle Ctrl+C"""
        print(f"\n[{datetime.now()}] Shutting down...")
        self.print_stats()
        self.running = False
        
    def print_stats(self):
        """Print statistics"""
        print("\n=== STATISTICS ===")
        print(f"Total signals received: {self.stats['total']}")
        print(f"Liquidity removals: {self.stats['liquidity_removal']}")
        print(f"Trading enabled: {self.stats['trading_enabled']}")
        
    def cleanup(self):
        """Clean up resources"""
        if self.subscriber:
            self.subscriber.close()
        self.context.term()
        print("Disconnected from ZMQ publisher")

def main():
    """Main entry point"""
    print("=" * 80)
    print("Mempool Signal Detector - ZMQ Subscriber Example")
    print("=" * 80)
    
    # Create and run subscriber
    subscriber = SignalSubscriber()
    subscriber.run()

if __name__ == "__main__":
    main()