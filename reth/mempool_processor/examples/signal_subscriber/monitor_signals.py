#!/usr/bin/env python3
"""Monitor both log files and ZMQ signals to verify publishing"""

import zmq
import json
import time
import os
from datetime import datetime
import threading

class SignalMonitor:
    def __init__(self):
        self.running = True
        self.signals_received = 0
        
    def monitor_logs(self):
        """Monitor log files for new detections"""
        log_dirs = [d for d in os.listdir("/home/nima/code/crypto/logs/mempool/") 
                   if d.startswith("signal_detector_2025")]
        
        if not log_dirs:
            print("No signal detector logs found")
            return
            
        latest_dir = sorted(log_dirs)[-1]
        log_path = f"/home/nima/code/crypto/logs/mempool/{latest_dir}"
        
        print(f"Monitoring logs in: {log_path}")
        
        liquidity_log = os.path.join(log_path, "liquidity_removals.log")
        trading_log = os.path.join(log_path, "trading_enabled.log")
        
        # Track file sizes
        liquidity_size = os.path.getsize(liquidity_log) if os.path.exists(liquidity_log) else 0
        trading_size = os.path.getsize(trading_log) if os.path.exists(trading_log) else 0
        
        while self.running:
            # Check liquidity removals
            if os.path.exists(liquidity_log):
                new_size = os.path.getsize(liquidity_log)
                if new_size > liquidity_size:
                    print(f"\n📁 NEW LOG: Liquidity removal detected in log file!")
                    with open(liquidity_log, 'r') as f:
                        f.seek(liquidity_size)
                        new_lines = f.read()
                        print(f"   {new_lines.strip()}")
                    liquidity_size = new_size
                    
            # Check trading enabled
            if os.path.exists(trading_log):
                new_size = os.path.getsize(trading_log)
                if new_size > trading_size:
                    print(f"\n📁 NEW LOG: Trading enabled detected in log file!")
                    with open(trading_log, 'r') as f:
                        f.seek(trading_size)
                        new_lines = f.read()
                        print(f"   {new_lines.strip()}")
                    trading_size = new_size
                    
            time.sleep(1)
            
    def monitor_zmq(self):
        """Monitor ZMQ for signals"""
        context = zmq.Context()
        subscriber = context.socket(zmq.SUB)
        
        try:
            subscriber.connect("tcp://127.0.0.1:5556")
            subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
            subscriber.setsockopt(zmq.RCVTIMEO, 1000)
            
            print("Connected to ZMQ publisher at tcp://127.0.0.1:5556")
            
            while self.running:
                try:
                    message = subscriber.recv_string()
                    self.signals_received += 1
                    signal = json.loads(message)
                    
                    print(f"\n📨 ZMQ SIGNAL #{self.signals_received}:")
                    print(f"   Type: {signal['alert_type']}")
                    print(f"   Function: {signal['function_name']}")
                    print(f"   TX: {signal['tx_hash']}")
                    
                except zmq.Again:
                    continue
                except Exception as e:
                    print(f"ZMQ Error: {e}")
                    
        finally:
            subscriber.close()
            context.term()
            
    def run(self):
        """Run both monitors"""
        print("=" * 80)
        print("Signal Monitor - Watching both logs and ZMQ")
        print("=" * 80)
        print("Press Ctrl+C to stop\n")
        
        # Start log monitor in thread
        log_thread = threading.Thread(target=self.monitor_logs)
        log_thread.daemon = True
        log_thread.start()
        
        # Run ZMQ monitor in main thread
        try:
            self.monitor_zmq()
        except KeyboardInterrupt:
            print("\nShutting down...")
            self.running = False

if __name__ == "__main__":
    monitor = SignalMonitor()
    monitor.run()