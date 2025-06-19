#!/usr/bin/env python3
"""
Signal Integration Test Tool
Tests connection to mempool processor alert publisher and validates messages
"""

import zmq
import json
import time
import sys
from datetime import datetime
from collections import defaultdict

class SignalIntegrationTester:
    def __init__(self, endpoint="tcp://localhost:5559"):
        self.endpoint = endpoint
        self.stats = defaultdict(int)
        self.alerts_by_type = defaultdict(list)
        
    def test_connection(self):
        """Test basic ZMQ connection"""
        print(f"Testing connection to {self.endpoint}...")
        
        try:
            context = zmq.Context()
            socket = context.socket(zmq.SUB)
            socket.connect(self.endpoint)
            socket.setsockopt_string(zmq.SUBSCRIBE, "")
            socket.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
            
            print("✅ Connected successfully")
            print("Waiting for alerts (5 second timeout)...")
            
            try:
                message = socket.recv_string()
                alert = json.loads(message)
                print("✅ Received valid alert!")
                self.display_alert(alert)
                return True
            except zmq.Again:
                print("⚠️  No alerts received in 5 seconds")
                print("Make sure mempool processor is running with --enable-publisher")
                return False
                
        except Exception as e:
            print(f"❌ Connection failed: {e}")
            return False
        finally:
            socket.close()
            context.term()
    
    def display_alert(self, alert):
        """Display alert in readable format"""
        print("\n" + "="*60)
        print(f"Alert ID: {alert.get('alert_id', 'N/A')}")
        print(f"Time: {datetime.fromtimestamp(alert.get('timestamp', 0))}")
        print(f"Type: {alert.get('event_type')} ({alert.get('severity')})")
        print(f"TX: {alert.get('tx_hash', 'N/A')[:20]}...")
        print(f"Pool: {alert.get('pool_address', 'N/A')[:20]}...")
        print(f"Token: {alert.get('token_symbol', 'N/A')} @ {alert.get('token_address', 'N/A')[:20]}...")
        print(f"ETH Change: {alert.get('eth_change_amount', 0):.4f} ({alert.get('eth_change_percent', 0):.1f}%)")
        print(f"Confidence: {alert.get('confidence_score', 0):.2f}")
        print(f"Details: {alert.get('details', 'N/A')}")
        print("="*60)
    
    def monitor_alerts(self, duration_seconds=60):
        """Monitor alerts for specified duration"""
        print(f"\nMonitoring alerts for {duration_seconds} seconds...")
        
        context = zmq.Context()
        socket = context.socket(zmq.SUB)
        socket.connect(self.endpoint)
        socket.setsockopt_string(zmq.SUBSCRIBE, "")
        
        start_time = time.time()
        last_alert_time = start_time
        
        try:
            while time.time() - start_time < duration_seconds:
                if socket.poll(1000):  # 1 second timeout
                    try:
                        recv_time = time.time()
                        message = socket.recv_string()
                        alert = json.loads(message)
                        
                        # Update stats
                        self.stats['total'] += 1
                        self.stats[alert['event_type']] += 1
                        self.alerts_by_type[alert['event_type']].append(alert)
                        
                        # Calculate latency
                        latency = (recv_time - last_alert_time) * 1000
                        last_alert_time = recv_time
                        
                        # Display summary
                        print(f"\r[{datetime.now().strftime('%H:%M:%S')}] "
                              f"Total: {self.stats['total']} | "
                              f"Scams: {self.stats.get('ScamAlert', 0)} | "
                              f"Warnings: {self.stats.get('LiquidityWarning', 0)} | "
                              f"Trades: {self.stats.get('LargeTrade', 0)} | "
                              f"Gap: {latency:.0f}ms", end='')
                        
                    except json.JSONDecodeError:
                        self.stats['decode_errors'] += 1
                    except Exception as e:
                        self.stats['errors'] += 1
                        print(f"\nError: {e}")
                else:
                    print(".", end='', flush=True)
                    
        except KeyboardInterrupt:
            print("\nMonitoring stopped by user")
        finally:
            socket.close()
            context.term()
            
        # Display summary
        self.display_summary()
    
    def display_summary(self):
        """Display monitoring summary"""
        print("\n\n" + "="*60)
        print("MONITORING SUMMARY")
        print("="*60)
        
        print(f"Total alerts: {self.stats['total']}")
        print(f"Decode errors: {self.stats.get('decode_errors', 0)}")
        print(f"Other errors: {self.stats.get('errors', 0)}")
        
        print("\nAlerts by type:")
        for event_type, count in self.stats.items():
            if event_type not in ['total', 'decode_errors', 'errors']:
                print(f"  {event_type}: {count}")
        
        # Show example of each type
        print("\nExample alerts:")
        for event_type, alerts in self.alerts_by_type.items():
            if alerts:
                print(f"\n{event_type} example:")
                self.display_alert(alerts[0])
                
        # Calculate performance metrics
        if self.stats['total'] > 0:
            print("\nPerformance metrics:")
            print(f"  Alert rate: {self.stats['total'] / 60:.2f} alerts/second")
            
            # Analyze severity distribution
            critical_count = sum(1 for alerts in self.alerts_by_type.values() 
                               for alert in alerts if alert.get('severity') == 'Critical')
            print(f"  Critical alerts: {critical_count} ({critical_count/self.stats['total']*100:.1f}%)")

def main():
    """Main test function"""
    print("=== Mempool Processor Signal Integration Test ===\n")
    
    tester = SignalIntegrationTester()
    
    # Test 1: Basic connection
    print("Test 1: Connection Test")
    if not tester.test_connection():
        print("\nTroubleshooting:")
        print("1. Check mempool processor is running:")
        print("   ps aux | grep mempool_signal.*enable-publisher")
        print("2. Verify --enable-publisher flag is used")
        print("3. Check port 5559 is not blocked")
        print("4. Try during active trading hours")
        sys.exit(1)
    
    # Test 2: Monitor for alerts
    print("\nTest 2: Alert Monitoring")
    print("Press Ctrl+C to stop monitoring")
    tester.monitor_alerts(duration_seconds=60)
    
    print("\n✅ Integration test complete!")

if __name__ == "__main__":
    main()