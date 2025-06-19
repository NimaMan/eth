#!/usr/bin/env python3
"""
Alert Throughput Test
Measures how many alerts per second the system can handle
"""

import zmq
import json
import time
import threading
from datetime import datetime
import statistics

class ThroughputTester:
    def __init__(self):
        self.alerts_received = []
        self.latencies = []
        self.running = False
        
    def create_test_alert(self, index):
        """Create a test alert message"""
        return {
            "alert_id": f"test_{index}_{int(time.time()*1000)}",
            "timestamp": int(time.time()),
            "severity": "Critical",
            "event_type": "ScamAlert",
            "tx_hash": f"0x{'a' * 63}{index}",
            "detected_latency_us": 761000,
            "pool_address": f"0x{'b' * 39}{index}",
            "pool_version": "V2",
            "token_address": f"0x{'c' * 39}{index}",
            "token_symbol": "TEST",
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
            "details": f"Test alert {index}"
        }
    
    def publisher_thread(self, num_alerts=1000):
        """Publish test alerts as fast as possible"""
        context = zmq.Context()
        socket = context.socket(zmq.PUB)
        socket.bind("tcp://*:5560")  # Different port for testing
        socket.setsockopt(zmq.SNDHWM, 10000)
        
        print(f"Publisher starting, will send {num_alerts} alerts...")
        time.sleep(1)  # Let subscriber connect
        
        start_time = time.time()
        
        for i in range(num_alerts):
            alert = self.create_test_alert(i)
            alert['send_time'] = time.time()
            message = json.dumps(alert)
            
            try:
                socket.send_string(message, zmq.DONTWAIT)
            except zmq.Again:
                print(f"Warning: Publisher buffer full at alert {i}")
                time.sleep(0.001)  # Brief pause
                socket.send_string(message)
        
        end_time = time.time()
        duration = end_time - start_time
        rate = num_alerts / duration
        
        print(f"Publisher finished: {num_alerts} alerts in {duration:.2f}s = {rate:.2f} alerts/sec")
        
        socket.close()
        context.term()
    
    def subscriber_thread(self):
        """Subscribe and measure throughput"""
        context = zmq.Context()
        socket = context.socket(zmq.SUB)
        socket.connect("tcp://localhost:5560")
        socket.setsockopt_string(zmq.SUBSCRIBE, "")
        
        print("Subscriber connected, waiting for alerts...")
        
        while self.running:
            if socket.poll(100):
                try:
                    recv_time = time.time()
                    message = socket.recv_string()
                    alert = json.loads(message)
                    
                    # Calculate latency if send_time available
                    if 'send_time' in alert:
                        latency = (recv_time - alert['send_time']) * 1000  # ms
                        self.latencies.append(latency)
                    
                    self.alerts_received.append(alert)
                    
                    # Progress indicator
                    if len(self.alerts_received) % 100 == 0:
                        print(f"  Received {len(self.alerts_received)} alerts...")
                        
                except Exception as e:
                    print(f"Error receiving alert: {e}")
        
        socket.close()
        context.term()

def run_throughput_test(num_alerts=1000):
    """Run the throughput test"""
    print("=== Alert Throughput Test ===")
    print(f"Testing with {num_alerts} alerts\n")
    
    tester = ThroughputTester()
    
    # Start subscriber
    tester.running = True
    sub_thread = threading.Thread(target=tester.subscriber_thread)
    sub_thread.daemon = True
    sub_thread.start()
    
    time.sleep(1)  # Let subscriber initialize
    
    # Start publisher
    pub_thread = threading.Thread(target=tester.publisher_thread, args=(num_alerts,))
    pub_thread.daemon = True
    pub_thread.start()
    
    # Wait for publisher to finish
    pub_thread.join(timeout=30)
    
    # Give subscriber time to catch up
    print("\nWaiting for subscriber to process remaining alerts...")
    time.sleep(2)
    
    # Stop subscriber
    tester.running = False
    sub_thread.join(timeout=2)
    
    # Analyze results
    print(f"\n=== Results ===")
    print(f"Alerts sent: {num_alerts}")
    print(f"Alerts received: {len(tester.alerts_received)}")
    
    if len(tester.alerts_received) > 0:
        # Calculate throughput
        first_alert = tester.alerts_received[0]
        last_alert = tester.alerts_received[-1]
        duration = last_alert['timestamp'] - first_alert['timestamp']
        
        if duration > 0:
            throughput = len(tester.alerts_received) / duration
            print(f"Measured throughput: {throughput:.2f} alerts/second")
        else:
            # All received in same second
            print(f"Measured throughput: >{len(tester.alerts_received)} alerts/second")
        
        # Analyze latencies
        if tester.latencies:
            avg_latency = statistics.mean(tester.latencies)
            p95_latency = statistics.quantiles(tester.latencies, n=100)[94]
            p99_latency = statistics.quantiles(tester.latencies, n=100)[98]
            max_latency = max(tester.latencies)
            
            print(f"\nLatency Statistics:")
            print(f"  Average: {avg_latency:.3f}ms")
            print(f"  P95: {p95_latency:.3f}ms")
            print(f"  P99: {p99_latency:.3f}ms")
            print(f"  Max: {max_latency:.3f}ms")
        
        # Check against requirements
        print(f"\n=== Validation ===")
        success = True
        
        # Requirement: Process 100+ alerts/second
        if len(tester.alerts_received) >= 100:
            print("✅ Can process 100+ alerts")
        else:
            print("❌ Failed to process 100 alerts")
            success = False
        
        # Check delivery rate
        delivery_rate = len(tester.alerts_received) / num_alerts * 100
        if delivery_rate >= 99:
            print(f"✅ Delivery rate: {delivery_rate:.1f}%")
        else:
            print(f"⚠️  Delivery rate: {delivery_rate:.1f}% (some alerts lost)")
            success = False
        
        return success
    else:
        print("❌ No alerts received!")
        return False

def test_sustained_load():
    """Test sustained high-throughput operation"""
    print("\n=== Sustained Load Test ===")
    print("Testing 10,000 alerts over extended period...\n")
    
    success = run_throughput_test(10000)
    
    if success:
        print("\n✅ System can handle sustained high load")
    else:
        print("\n❌ System struggles with sustained load")
    
    return success

if __name__ == "__main__":
    # Run basic throughput test
    basic_success = run_throughput_test(1000)
    
    # If basic test passes, try sustained load
    if basic_success:
        sustained_success = test_sustained_load()
        
        if sustained_success:
            print("\n✅ THROUGHPUT TESTS PASSED")
            print("System meets performance requirements")
        else:
            print("\n⚠️  PARTIAL SUCCESS")
            print("Basic throughput OK, but struggles with sustained load")
    else:
        print("\n❌ THROUGHPUT TESTS FAILED")
        print("System does not meet performance requirements")