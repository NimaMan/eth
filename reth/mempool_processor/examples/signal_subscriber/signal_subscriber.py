#!/usr/bin/env python3
"""
Simple ZMQ Signal Subscriber for Testing Mempool Processor Signals

This subscriber connects to the mempool processor's signal publisher and
displays received signals. It supports multiple concurrent consumers since
ZMQ SUB/PUB pattern allows multiple subscribers.

Usage:
    python signal_subscriber.py

Environment Variables:
    SIGNAL_ENDPOINT: ZMQ endpoint (default: tcp://127.0.0.1:5557)
"""

import zmq
import json
import signal
import sys
from datetime import datetime
import os

class SignalSubscriber:
    def __init__(self, endpoint=None):
        self.endpoint = endpoint or os.getenv("SIGNAL_ENDPOINT", "tcp://127.0.0.1:5557")
        self.context = zmq.Context()
        self.socket = None
        self.running = True
        self.stats = {
            "total": 0,
            "liquidity": 0, 
            "trading": 0,
            "tax": 0,
            "stablecoin": 0,
            "lp_approval": 0
        }
        
    def connect(self):
        """Connect to signal publisher"""
        self.socket = self.context.socket(zmq.SUB)
        self.socket.connect(self.endpoint)
        
        # Subscribe to all messages (empty subscription = all)
        self.socket.setsockopt(zmq.SUBSCRIBE, b"")
        
        # Set receive timeout to allow graceful shutdown
        self.socket.setsockopt(zmq.RCVTIMEO, 1000)
        
        print(f"📡 Connected to signal publisher: {self.endpoint}")
        print("🔄 Waiting for signals... (Press Ctrl+C to stop)\n")
        
    def handle_signal(self, message):
        """Process received signal message"""
        try:
            # Try to parse as JSON first
            if message.strip().startswith('{'):
                signal_data = json.loads(message)
                self.handle_json_signal(signal_data)
            else:
                # Handle text-based signals (like current tax signals)
                self.handle_text_signal(message)
                
        except json.JSONDecodeError:
            # Not JSON, treat as text signal
            self.handle_text_signal(message)
        except Exception as e:
            print(f"❌ Error processing signal: {e}")
            print(f"   Raw message: {message}")
            
    def handle_json_signal(self, data):
        """Handle JSON-formatted signals"""
        self.stats["total"] += 1
        signal_type = data.get("type", "unknown")
        
        if "liquidity" in signal_type.lower():
            self.stats["liquidity"] += 1
        elif "trading" in signal_type.lower():
            self.stats["trading"] += 1
        elif "tax" in signal_type.lower():
            self.stats["tax"] += 1
            
        print(f"📨 JSON Signal #{self.stats['total']} [{datetime.now().strftime('%H:%M:%S')}]")
        print(f"   Type: {signal_type}")
        print(f"   Data: {json.dumps(data, indent=2)}")
        print("-" * 60)
        
    def handle_text_signal(self, message):
        """Handle text-based signals (current format)"""
        self.stats["total"] += 1
        
        # Detect signal type from message content
        if "TAX_DETECTION" in message or "TAX_SIGNAL" in message:
            self.stats["tax"] += 1
            signal_type = "TAX"
        elif "LIQUIDITY" in message:
            self.stats["liquidity"] += 1 
            signal_type = "LIQUIDITY"
        elif "TRADING" in message:
            self.stats["trading"] += 1
            signal_type = "TRADING"
        elif "STABLECOIN" in message:
            self.stats["stablecoin"] += 1
            signal_type = "STABLECOIN"
        elif "LP_APPROVAL" in message:
            self.stats["lp_approval"] += 1
            signal_type = "LP_APPROVAL"
        else:
            signal_type = "OTHER"
            
        print(f"📨 {signal_type} Signal #{self.stats['total']} [{datetime.now().strftime('%H:%M:%S')}]")
        print(f"   {message.strip()}")
        print("-" * 60)
        
    def run(self):
        """Main subscriber loop"""
        # Setup signal handler for graceful shutdown
        signal.signal(signal.SIGINT, self._signal_handler)
        
        self.connect()
        
        while self.running:
            try:
                # Receive message with timeout
                message = self.socket.recv_string(zmq.NOBLOCK)
                self.handle_signal(message)
                
            except zmq.Again:
                # No message received within timeout, continue
                continue
            except zmq.ZMQError as e:
                if e.errno == zmq.ETERM:
                    break
                print(f"❌ ZMQ Error: {e}")
                break
                
        self.cleanup()
        
    def _signal_handler(self, signum, frame):
        """Handle Ctrl+C gracefully"""
        print(f"\n🛑 Received signal {signum}, shutting down...")
        self.running = False
        
    def cleanup(self):
        """Clean up resources"""
        print("\n📊 Final Statistics:")
        print(f"   Total signals: {self.stats['total']}")
        print(f"   Tax signals: {self.stats['tax']}")
        print(f"   Liquidity signals: {self.stats['liquidity']}")
        print(f"   Trading signals: {self.stats['trading']}")
        print(f"   Stablecoin signals: {self.stats['stablecoin']}")
        print(f"   LP Approval signals: {self.stats['lp_approval']}")
        
        if self.socket:
            self.socket.close()
        self.context.term()
        print("✅ Disconnected from signal publisher")

def main():
    print("=" * 70)
    print("🚀 Mempool Processor - Signal Subscriber")
    print("=" * 70)
    print("This subscriber can run alongside other consumers like:")
    print("  • Database writers")
    print("  • eth_kartal signal processor")
    print("  • Other monitoring tools")
    print("=" * 70)
    
    subscriber = SignalSubscriber()
    subscriber.run()

if __name__ == "__main__":
    main()