#!/usr/bin/env python3
"""
Unified Publisher Python Subscriber

Algorithm:
1. Connect to unified publisher on port 5560
2. Subscribe to specific topics (or all)
3. Receive multipart messages [topic, json_data]
4. Parse and display signals
5. Handle different signal types appropriately
"""

import zmq
import json
import sys
from datetime import datetime
from typing import Dict, List, Optional

class UnifiedSignalSubscriber:
    """Subscribe to unified signals from Rust mempool processor"""
    
    def __init__(self, endpoint: str = "tcp://127.0.0.1:5560", topics: List[str] = None):
        self.endpoint = endpoint
        self.topics = topics or []
        self.context = zmq.Context()
        self.socket = None
        self.stats = {
            'total': 0,
            'tax_manipulation': 0,
            'liquidity_removal': 0,
            'pool_drain': 0,
            'trading_enabled': 0,
            'scam_alert': 0,
        }
        
    def connect(self):
        """Connect to publisher and subscribe to topics"""
        self.socket = self.context.socket(zmq.SUB)
        self.socket.connect(self.endpoint)
        
        if not self.topics:
            # Subscribe to all
            self.socket.setsockopt_string(zmq.SUBSCRIBE, "")
            print(f"📡 Subscribed to ALL topics on {self.endpoint}")
        else:
            # Subscribe to specific topics
            for topic in self.topics:
                self.socket.setsockopt_string(zmq.SUBSCRIBE, topic)
                print(f"📡 Subscribed to topic: {topic}")
                
    def handle_signal(self, topic: str, signal: Dict):
        """Process and display a signal"""
        self.stats['total'] += 1
        if topic in self.stats:
            self.stats[topic] += 1
            
        # Display header
        print(f"\n{'='*80}")
        print(f"📨 SIGNAL RECEIVED! #{self.stats['total']}")
        print(f"Topic: {topic.upper()}")
        print(f"Type: {signal.get('signal_type')}")
        print(f"Severity: {signal.get('severity')}")
        print(f"Confidence: {signal.get('confidence', 0):.2%}")
        print(f"TX: {signal.get('tx_hash')}")
        print(f"Token: {signal.get('token_address')}")
        
        # Handle type-specific data
        data = signal.get('data', {})
        data_type = data.get('data_type')
        data_content = data.get('data', {})
        
        if data_type == 'TaxManipulation':
            self._handle_tax_manipulation(data_content)
        elif data_type == 'LiquidityRemoval':
            self._handle_liquidity_removal(data_content)
        elif data_type == 'PoolDrain':
            self._handle_pool_drain(data_content)
        elif data_type == 'TradingStatus':
            self._handle_trading_status(data_content)
        else:
            print(f"Details: {signal.get('details')}")
            
        print(f"Timestamp: {datetime.fromtimestamp(signal.get('timestamp', 0))}")
        print('='*80)
        
    def _handle_tax_manipulation(self, data: Dict):
        """Handle tax manipulation signals"""
        print("\n🚨 TAX MANIPULATION DETECTED!")
        print(f"  Pattern: {data.get('pattern')}")
        print(f"  Buy tax: {data.get('current_buy_tax')}% → {data.get('predicted_buy_tax')}%")
        print(f"  Sell tax: {data.get('current_sell_tax')}% → {data.get('predicted_sell_tax')}%")
        print(f"  Manipulator: {data.get('manipulator_address')}")
        print(f"  Function: {data.get('function_selector')}")
        
        # Determine risk level
        sell_tax = data.get('predicted_sell_tax', 0)
        if sell_tax > 90:
            print("  ⚠️  HONEYPOT RISK: Sell tax > 90%!")
        elif sell_tax > 50:
            print("  ⚠️  HIGH RISK: Sell tax > 50%")
            
    def _handle_liquidity_removal(self, data: Dict):
        """Handle liquidity removal signals"""
        print("\n💧 LIQUIDITY REMOVAL!")
        print(f"  Function: {data.get('function_name')}")
        if data.get('percentage'):
            print(f"  Percentage: {data['percentage']:.1f}%")
        if data.get('eth_amount'):
            print(f"  ETH amount: {data['eth_amount']:.4f}")
        if data.get('token_amount'):
            print(f"  Token amount: {data['token_amount']:,.2f}")
            
    def _handle_pool_drain(self, data: Dict):
        """Handle pool drain signals"""
        print("\n🚨🚨 POOL DRAIN ALERT! 🚨🚨")
        print(f"  ETH drained: {data.get('eth_drained', 0):.4f} ({data.get('drain_percentage', 0):.1f}%)")
        print(f"  Remaining ETH: {data.get('new_eth_reserve', 0):.4f}")
        print(f"  Token reserve: {data.get('current_token_reserve', 0):,.0f} → {data.get('new_token_reserve', 0):,.0f}")
        if data.get('is_complete_drain'):
            print("  ⚠️⚠️  COMPLETE DRAIN - RUGPULL IN PROGRESS! ⚠️⚠️")
            
    def _handle_trading_status(self, data: Dict):
        """Handle trading status signals"""
        print("\n📊 TRADING STATUS CHANGE!")
        print(f"  Function: {data.get('function_name')}")
        print(f"  Enabled: {data.get('enabled')}")
        if data.get('pool_has_liquidity'):
            print(f"  Pool has liquidity: Yes")
            if data.get('initial_liquidity_eth'):
                print(f"  Initial ETH: {data['initial_liquidity_eth']:.4f}")
            if data.get('initial_liquidity_tokens'):
                print(f"  Initial tokens: {data['initial_liquidity_tokens']:,.2f}")
                
    def print_stats(self):
        """Display statistics"""
        print("\n" + "="*80)
        print("STATISTICS")
        print(f"Total signals received: {self.stats['total']}")
        print("\nBy type:")
        for signal_type, count in self.stats.items():
            if signal_type != 'total' and count > 0:
                print(f"  - {signal_type}: {count}")
        print("="*80)
        
    def run(self):
        """Main subscription loop"""
        print(f"🚀 Starting Unified Signal Subscriber")
        print(f"Endpoint: {self.endpoint}")
        print(f"Press Ctrl+C to stop\n")
        
        self.connect()
        
        try:
            while True:
                # Receive multipart message
                try:
                    parts = self.socket.recv_multipart()
                    if len(parts) != 2:
                        print(f"⚠️  Invalid message format: expected 2 parts, got {len(parts)}")
                        continue
                        
                    topic = parts[0].decode('utf-8')
                    json_data = parts[1].decode('utf-8')
                    
                    # Parse signal
                    signal = json.loads(json_data)
                    self.handle_signal(topic, signal)
                    
                except zmq.Again:
                    # Timeout, no message
                    continue
                except json.JSONDecodeError as e:
                    print(f"❌ JSON decode error: {e}")
                    print(f"Raw data: {json_data[:200]}...")
                except Exception as e:
                    print(f"❌ Error processing message: {e}")
                    
        except KeyboardInterrupt:
            print("\n\n🛑 Shutting down...")
            self.print_stats()
        finally:
            self.socket.close()
            self.context.term()


def main():
    """Main entry point"""
    # Parse command line arguments
    topics = sys.argv[1:] if len(sys.argv) > 1 else []
    
    if topics:
        print(f"Subscribing to topics: {topics}")
    else:
        print("Subscribing to ALL topics")
        
    # Create and run subscriber
    subscriber = UnifiedSignalSubscriber(topics=topics)
    subscriber.run()


if __name__ == "__main__":
    main()