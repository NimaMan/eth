#!/usr/bin/env python3
"""
Simulate Real Scams Test

This test replays actual scam transactions from the mempool processor logs
to validate that ETH Kartal would have protected users.

It tests:
1. Real scam pattern recognition
2. Appropriate strategy selection
3. Timing requirements for protection
4. Expected profit/loss calculations
"""

import json
import time
import zmq
from datetime import datetime
import re

class ScamSimulator:
    def __init__(self, log_file="/home/nima/code/crypto/logs/mempool/market_events_full_tx_20250619_133220.log"):
        self.log_file = log_file
        self.scam_alerts = []
        self.warning_alerts = []
        
    def parse_log_file(self):
        """Parse real scam alerts from mempool processor log"""
        print(f"Parsing log file: {self.log_file}")
        
        with open(self.log_file, 'r') as f:
            for line in f:
                if '🚨 ScamAlert' in line:
                    alert = self._parse_scam_line(line, 'Critical')
                    if alert:
                        self.scam_alerts.append(alert)
                elif '⚠️ LiquidityWarning' in line:
                    alert = self._parse_scam_line(line, 'Warning')
                    if alert:
                        self.warning_alerts.append(alert)
        
        print(f"Found {len(self.scam_alerts)} ScamAlerts and {len(self.warning_alerts)} Warnings")
        
    def _parse_scam_line(self, line, severity):
        """Parse a single log line into alert structure"""
        try:
            # Extract timestamp
            timestamp_match = re.search(r'\[([\d-\s:\.]+)\]', line)
            timestamp = timestamp_match.group(1) if timestamp_match else None
            
            # Parse fields using regex
            fields = {}
            patterns = {
                'tx_hash': r'TX: (0x[a-fA-F0-9]+)',
                'pool_address': r'Pool: (0x[a-fA-F0-9]+)',
                'token_address': r'Token: (0x[a-fA-F0-9]+)',
                'eth_values': r'ETH: ([\d\.]+) -> ([\d\.]+)',
                'loss_eth': r'Lost: ([\d\.]+) ETH',
                'ipc_latency': r'IPC: ([\d\.]+)ms',
                'percent_change': r'\((-?[\d\.]+)% loss\)',
            }
            
            for field, pattern in patterns.items():
                match = re.search(pattern, line)
                if match:
                    if field == 'eth_values':
                        fields['current_eth'] = float(match.group(1))
                        fields['simulated_eth'] = float(match.group(2))
                    elif field == 'loss_eth':
                        fields['eth_lost'] = float(match.group(1))
                    elif field == 'ipc_latency':
                        fields['detection_latency_ms'] = float(match.group(1))
                    elif field == 'percent_change':
                        fields['percent_change'] = float(match.group(1))
                    else:
                        fields[field] = match.group(1)
            
            # Build alert structure
            alert = {
                'alert_id': f"real_{fields.get('tx_hash', 'unknown')[:10]}_{int(time.time())}",
                'severity': severity,
                'timestamp': timestamp,
                'tx_hash': fields.get('tx_hash', ''),
                'pool_address': fields.get('pool_address', ''),
                'token_address': fields.get('token_address', ''),
                'current_eth_reserve': fields.get('current_eth', 0),
                'simulated_eth_reserve': fields.get('simulated_eth', 0),
                'eth_change_amount': -fields.get('eth_lost', 0),
                'eth_change_percent': -abs(fields.get('percent_change', 0)) / 100,  # Convert to decimal
                'detection_latency_us': int(fields.get('detection_latency_ms', 0) * 1000),
                'confidence_score': 0.99 if severity == 'Critical' else 0.85,
                'gas_price_gwei': 30.0,  # Estimate
            }
            
            return alert
            
        except Exception as e:
            print(f"Error parsing line: {e}")
            return None
    
    def analyze_patterns(self):
        """Analyze patterns in real scams"""
        print("\n=== SCAM PATTERN ANALYSIS ===")
        
        # Analyze 100% drains
        total_drains = [a for a in self.scam_alerts if a['simulated_eth_reserve'] == 0]
        print(f"\n100% Liquidity Drains: {len(total_drains)}")
        
        if total_drains:
            eth_amounts = [a['current_eth_reserve'] for a in total_drains]
            print(f"  Min pool size: {min(eth_amounts):.6f} ETH")
            print(f"  Max pool size: {max(eth_amounts):.6f} ETH")
            print(f"  Avg pool size: {sum(eth_amounts)/len(eth_amounts):.6f} ETH")
            print(f"  Total ETH drained: {sum(eth_amounts):.6f} ETH")
        
        # Analyze partial drains
        partial_drains = [a for a in self.warning_alerts if a['eth_change_percent'] < -0.2]
        print(f"\nPartial Drains (>20%): {len(partial_drains)}")
        
        if partial_drains:
            percentages = [abs(a['eth_change_percent'] * 100) for a in partial_drains]
            print(f"  Min drain: {min(percentages):.1f}%")
            print(f"  Max drain: {max(percentages):.1f}%")
            print(f"  Avg drain: {sum(percentages)/len(percentages):.1f}%")
        
        # Timing analysis
        all_alerts = self.scam_alerts + self.warning_alerts
        latencies = [a['detection_latency_us'] / 1000 for a in all_alerts if a['detection_latency_us'] > 0]
        
        if latencies:
            print(f"\nDetection Latencies:")
            print(f"  Min: {min(latencies):.3f}ms")
            print(f"  Max: {max(latencies):.3f}ms")
            print(f"  Avg: {sum(latencies)/len(latencies):.3f}ms")
    
    def simulate_protection(self, alert):
        """Simulate what ETH Kartal would do for this alert"""
        result = {
            'alert_id': alert['alert_id'],
            'strategy': None,
            'action': None,
            'estimated_save': 0,
            'gas_cost_eth': 0,
            'net_profit': 0,
            'execution_time_ms': 0,
            'success': False
        }
        
        # Determine strategy based on severity and drain percentage
        drain_percent = abs(alert['eth_change_percent'] * 100)
        
        if drain_percent >= 80:
            result['strategy'] = 'EmergencySell'
            result['action'] = 'SELL_ALL'
            # Assume we could save 90% with 30% slippage
            result['estimated_save'] = alert['current_eth_reserve'] * 0.7
            result['execution_time_ms'] = 150  # Fast execution
            
        elif drain_percent >= 50:
            result['strategy'] = 'PartialExit'
            result['action'] = 'SELL_75%'
            # Sell 75% of position with 15% slippage
            result['estimated_save'] = alert['current_eth_reserve'] * 0.75 * 0.85
            result['execution_time_ms'] = 180
            
        elif drain_percent >= 30:
            result['strategy'] = 'PartialExit'
            result['action'] = 'SELL_50%'
            # Sell 50% of position with 10% slippage
            result['estimated_save'] = alert['current_eth_reserve'] * 0.5 * 0.9
            result['execution_time_ms'] = 200
            
        else:
            result['strategy'] = 'Monitor'
            result['action'] = 'NO_ACTION'
            result['execution_time_ms'] = 0
            
        # Calculate gas costs (assuming high gas for emergency)
        if result['strategy'] != 'Monitor':
            gas_price_gwei = 50 if result['strategy'] == 'EmergencySell' else 30
            gas_used = 300000  # Typical swap gas
            result['gas_cost_eth'] = (gas_price_gwei * gas_used) / 1e9
            
            # Calculate net profit
            result['net_profit'] = result['estimated_save'] - result['gas_cost_eth'] - abs(alert['eth_change_amount'])
            result['success'] = result['net_profit'] > 0
            
        return result
    
    def send_to_kartal(self, alerts, delay_ms=100):
        """Send alerts to ETH Kartal via ZMQ"""
        context = zmq.Context()
        socket = context.socket(zmq.PUB)
        socket.bind("tcp://localhost:5558")
        time.sleep(1)  # Let socket bind
        
        print(f"\nSending {len(alerts)} alerts to ETH Kartal...")
        
        for i, alert in enumerate(alerts):
            socket.send_json(alert)
            print(f"  Sent {i+1}/{len(alerts)}: {alert['alert_id']}")
            time.sleep(delay_ms / 1000.0)
        
        socket.close()
        context.term()
    
    def run_simulation(self, limit=None):
        """Run full simulation"""
        print("\n=== PROTECTION SIMULATION ===")
        
        # Take first N alerts or all
        test_alerts = (self.scam_alerts + self.warning_alerts)[:limit]
        
        total_eth_at_risk = 0
        total_eth_saved = 0
        total_gas_cost = 0
        successful_protections = 0
        
        for alert in test_alerts:
            result = self.simulate_protection(alert)
            
            if result['strategy'] != 'Monitor':
                total_eth_at_risk += alert['current_eth_reserve']
                total_eth_saved += result['estimated_save']
                total_gas_cost += result['gas_cost_eth']
                
                if result['success']:
                    successful_protections += 1
                
                print(f"\nAlert: {alert['alert_id']}")
                print(f"  Pool: {alert['current_eth_reserve']:.4f} ETH at risk")
                print(f"  Strategy: {result['strategy']} - {result['action']}")
                print(f"  Estimated Save: {result['estimated_save']:.4f} ETH")
                print(f"  Gas Cost: {result['gas_cost_eth']:.4f} ETH")
                print(f"  Net Profit: {result['net_profit']:.4f} ETH")
                print(f"  Success: {'✅' if result['success'] else '❌'}")
        
        # Summary
        print("\n=== SIMULATION SUMMARY ===")
        print(f"Total Alerts Processed: {len(test_alerts)}")
        print(f"Total ETH at Risk: {total_eth_at_risk:.4f} ETH")
        print(f"Total ETH Saved: {total_eth_saved:.4f} ETH")
        print(f"Total Gas Cost: {total_gas_cost:.4f} ETH")
        print(f"Net ETH Saved: {total_eth_saved - total_gas_cost:.4f} ETH")
        print(f"Success Rate: {successful_protections}/{len(test_alerts)} ({successful_protections/len(test_alerts)*100:.1f}%)")
        
        if total_eth_at_risk > 0:
            save_rate = (total_eth_saved - total_gas_cost) / total_eth_at_risk * 100
            print(f"Save Rate: {save_rate:.1f}% of at-risk ETH")

def main():
    # Create simulator
    simulator = ScamSimulator()
    
    # Parse real alerts
    simulator.parse_log_file()
    
    # Analyze patterns
    simulator.analyze_patterns()
    
    # Run protection simulation
    simulator.run_simulation(limit=20)  # Test first 20
    
    # Optionally send to live ETH Kartal
    send_to_kartal = input("\nSend alerts to ETH Kartal? (y/n): ")
    if send_to_kartal.lower() == 'y':
        # Send first 5 alerts as test
        test_alerts = (simulator.scam_alerts + simulator.warning_alerts)[:5]
        simulator.send_to_kartal(test_alerts)

if __name__ == "__main__":
    main()