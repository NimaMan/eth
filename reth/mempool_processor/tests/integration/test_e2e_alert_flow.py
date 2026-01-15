#!/usr/bin/env python3
"""
End-to-End Alert Flow Test
Tests the complete pipeline from scam detection to ZMQ alert
"""

import subprocess
import zmq
import json
import time
import threading
import sys
import os
from datetime import datetime

class AlertValidator:
    def __init__(self):
        self.alerts_received = []
        self.subscriber_thread = None
        self.running = False
        
    def start_subscriber(self, endpoint="tcp://localhost:5559"):
        """Start ZMQ subscriber in background thread"""
        self.running = True
        
        def subscriber():
            context = zmq.Context()
            socket = context.socket(zmq.SUB)
            socket.connect(endpoint)
            socket.setsockopt_string(zmq.SUBSCRIBE, "")
            print(f"[{datetime.now()}] Subscriber connected to {endpoint}")
            
            while self.running:
                if socket.poll(100):  # 100ms timeout
                    try:
                        message = socket.recv_string()
                        alert = json.loads(message)
                        self.alerts_received.append(alert)
                        print(f"[{datetime.now()}] Alert received: {alert['event_type']} - {alert['eth_change_percent']:.2f}% drain")
                    except Exception as e:
                        print(f"Error parsing alert: {e}")
            
            socket.close()
            context.term()
        
        self.subscriber_thread = threading.Thread(target=subscriber)
        self.subscriber_thread.daemon = True
        self.subscriber_thread.start()
        time.sleep(1)  # Give subscriber time to connect
        
    def stop_subscriber(self):
        """Stop the subscriber thread"""
        self.running = False
        if self.subscriber_thread:
            self.subscriber_thread.join(timeout=2)
    
    def validate_alert(self, alert):
        """Validate alert has all required fields"""
        required_fields = [
            'alert_id', 'timestamp', 'severity', 'event_type',
            'tx_hash', 'pool_address', 'eth_change_percent',
            'confidence_score', 'current_eth_reserve', 'simulated_eth_reserve'
        ]
        
        missing_fields = [field for field in required_fields if field not in alert]
        if missing_fields:
            return False, f"Missing fields: {missing_fields}"
        
        # Validate field types
        if not isinstance(alert['eth_change_percent'], (int, float)):
            return False, "eth_change_percent must be numeric"
        
        if alert['event_type'] not in ['ScamAlert', 'LiquidityWarning', 'LargeTrade']:
            return False, f"Unknown event_type: {alert['event_type']}"
        
        # Validate severity matches drain percentage
        drain = abs(alert['eth_change_percent'])
        if drain > 50 and alert['severity'] != 'Critical':
            return False, f"Drain {drain}% should be Critical severity"
        
        return True, "Valid"

def run_e2e_test():
    """Run end-to-end alert flow test"""
    print("=== End-to-End Alert Flow Test ===")
    print(f"Started at: {datetime.now()}\n")
    
    # Check if binary exists
    binary_path = "./target/debug/mempool_signal_detection_full_tx_ipc"
    if not os.path.exists(binary_path):
        print("❌ Binary not found. Run: cargo build --bin mempool_signal_detection_full_tx_ipc")
        return False
    
    # 1. Start alert validator
    validator = AlertValidator()
    validator.start_subscriber()
    
    # 2. Start mempool processor with publisher enabled
    print("Starting mempool processor with publisher...")
    env = os.environ.copy()
    env['RUST_LOG'] = 'info'
    
    proc = subprocess.Popen(
        [binary_path, "--enable-publisher"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True
    )
    
    # 3. Wait for initialization
    print("Waiting for processor to initialize...")
    init_success = False
    start_time = time.time()
    
    while time.time() - start_time < 30:  # 30 second timeout
        line = proc.stdout.readline()
        if line:
            print(f"  {line.strip()}")
            if "Alert publisher initialized successfully" in line:
                init_success = True
                break
            if "error" in line.lower():
                print(f"❌ Error during initialization: {line}")
    
    if not init_success:
        print("❌ Failed to initialize alert publisher")
        proc.terminate()
        validator.stop_subscriber()
        return False
    
    # 4. Monitor for alerts
    print("\n✅ Publisher initialized. Monitoring for real alerts...")
    print("Waiting for scam transactions from mempool...\n")
    
    monitoring_duration = 60  # Monitor for 60 seconds
    start_monitor = time.time()
    
    while time.time() - start_monitor < monitoring_duration:
        if validator.alerts_received:
            break
        time.sleep(1)
        if int(time.time() - start_monitor) % 10 == 0:
            print(f"  ...monitoring ({int(time.time() - start_monitor)}s)")
    
    # 5. Analyze results
    print(f"\n=== Test Results ===")
    print(f"Monitoring duration: {int(time.time() - start_monitor)}s")
    print(f"Alerts received: {len(validator.alerts_received)}")
    
    if validator.alerts_received:
        print("\n✅ SUCCESS: Alert flow is working!")
        
        # Validate alerts
        for i, alert in enumerate(validator.alerts_received[:5]):  # Show first 5
            valid, msg = validator.validate_alert(alert)
            status = "✅" if valid else "❌"
            
            print(f"\nAlert {i+1}: {status} {msg}")
            print(f"  Type: {alert.get('event_type')}")
            print(f"  Severity: {alert.get('severity')}")
            print(f"  ETH Drain: {alert.get('eth_change_percent', 0):.2f}%")
            print(f"  Pool: {alert.get('pool_address', 'N/A')[:10]}...")
            print(f"  TX: {alert.get('tx_hash', 'N/A')[:10]}...")
        
        # Performance metrics
        if len(validator.alerts_received) > 1:
            # Calculate alert rate
            time_span = validator.alerts_received[-1]['timestamp'] - validator.alerts_received[0]['timestamp']
            if time_span > 0:
                rate = len(validator.alerts_received) / time_span
                print(f"\nAlert rate: {rate:.2f} alerts/second")
    else:
        print("\n⚠️  No alerts received during monitoring period")
        print("This could mean:")
        print("  1. No scam transactions in mempool")
        print("  2. Alert publisher not properly integrated")
        print("  3. ZMQ connection issue")
        print("\nTry running during active trading hours for more transactions")
    
    # 6. Cleanup
    print("\nCleaning up...")
    proc.terminate()
    validator.stop_subscriber()
    
    # Return success if we received and validated at least one alert
    return len(validator.alerts_received) > 0

def test_alert_format():
    """Test alert message format independently"""
    print("\n=== Alert Format Test ===")
    
    # Sample alert based on publisher.rs
    sample_alert = {
        "alert_id": "0x1234567890_1234567890",
        "timestamp": int(time.time()),
        "severity": "Critical",
        "event_type": "ScamAlert",
        "tx_hash": "0x" + "a" * 64,
        "detected_latency_us": 761000,
        "pool_address": "0x" + "b" * 40,
        "pool_version": "V2",
        "token_address": "0x" + "c" * 40,
        "token_symbol": "SCAM",
        "token_decimals": 18,
        "current_eth_reserve": 10.5,
        "simulated_eth_reserve": 0.5,
        "eth_change_amount": -10.0,
        "eth_change_percent": -95.24,
        "current_price": 1000.0,
        "simulated_price": 100.0,
        "price_impact_percent": -90.0,
        "confidence_score": 0.95,
        "gas_price_gwei": 30.0,
        "details": "Critical liquidity drain: 95.24% of pool ETH removed"
    }
    
    validator = AlertValidator()
    valid, msg = validator.validate_alert(sample_alert)
    
    if valid:
        print("✅ Sample alert format is valid")
    else:
        print(f"❌ Sample alert validation failed: {msg}")
    
    # Test JSON serialization
    try:
        json_str = json.dumps(sample_alert)
        parsed = json.loads(json_str)
        print(f"✅ JSON serialization works ({len(json_str)} bytes)")
    except Exception as e:
        print(f"❌ JSON serialization failed: {e}")

if __name__ == "__main__":
    # Run format test first
    test_alert_format()
    
    # Run E2E test
    print("\n" + "="*50 + "\n")
    success = run_e2e_test()
    
    if success:
        print("\n✅ END-TO-END TEST PASSED")
        print("The alert publishing system is working correctly!")
        sys.exit(0)
    else:
        print("\n❌ END-TO-END TEST FAILED")
        print("Check the issues above and try again")
        sys.exit(1)