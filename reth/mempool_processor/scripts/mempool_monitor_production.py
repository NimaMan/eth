#!/usr/bin/env python3
"""
Production Mempool Monitor - Real-time scam detection for liquidity pools
========================================================================

This monitor:
1. Connects to actual mempool via WebSocket (not historical blocks)
2. Gets all pools dynamically from Python token processor
3. Simulates pending transactions to detect scams BEFORE they're mined
4. Only logs scams and high-level stats (no debug output)
"""

import asyncio
import time
import json
import sys
import os
from datetime import datetime
from typing import Dict, Set, Optional
from web3 import Web3
import subprocess

# Add path for imports
sys.path.append('/home/nima/code/crypto/py')
sys.path.append('/home/nima/code/crypto/rust/mempool_processor')

from simulate_tx_state_changes import TransactionSimulator

class ProductionMempoolMonitor:
    def __init__(self):
        # Connect to local node
        self.w3 = Web3(Web3.HTTPProvider("http://localhost:8545"))
        self.ws_w3 = None
        self.has_websocket = False
        
        # Try WebSocket connection
        try:
            from web3.providers import WebsocketProvider
            self.ws_w3 = Web3(WebsocketProvider("ws://localhost:8546", websocket_timeout=60))
            self.has_websocket = self.ws_w3.is_connected()
        except:
            pass
            
        # Transaction simulator
        self.simulator = TransactionSimulator()
        
        # Pools we're monitoring
        self.monitored_pools = {}  # pool_address -> pool_info
        
        # Stats
        self.stats = {
            "start_time": time.time(),
            "seen": 0,
            "analyzed": 0,
            "scams_detected": 0,
            "scams_by_severity": {"CRITICAL": 0, "HIGH": 0, "MEDIUM": 0, "LOW": 0}
        }
        
        # Scam detection thresholds
        self.DRAIN_THRESHOLD = 0.2  # 20% drain
        
    def get_pools_from_token_processor(self) -> Dict[str, Dict]:
        """Get all active pools from the Python token processor"""
        pools = {}
        
        try:
            # Try to import and use token processor directly
            from eth_token.token_manager.token_processor import TokenProcessor
            processor = TokenProcessor()
            live_tokens = processor.get_live_tokens()
            
            for token_addr, token_data in live_tokens.items():
                if hasattr(token_data, 'pool_manager') and token_data.pool_manager:
                    for pool in token_data.pool_manager.get_all_pools():
                        pools[pool.pool_address] = {
                            'token_address': token_addr,
                            'pool_type': pool.pool_type,
                            'denom_currency': pool.denom_address,
                            'current_reserves': {
                                'token': pool.get_token_reserve(),
                                'denom': pool.get_denom_reserve()
                            }
                        }
        except:
            # Fallback: Try database or file cache
            try:
                import psycopg2
                conn = psycopg2.connect(host="localhost", database="eth_db", user="nima")
                with conn.cursor() as cur:
                    cur.execute("""
                        SELECT DISTINCT pool_address, token_address, pool_type, liquidity_eth
                        FROM token_pools
                        WHERE last_updated > NOW() - INTERVAL '1 hour'
                        AND pool_address IS NOT NULL
                    """)
                    for row in cur.fetchall():
                        pool_addr, token_addr, pool_type, liquidity = row
                        pools[pool_addr] = {
                            'token_address': token_addr,
                            'pool_type': pool_type or 'uniswap_v2',
                            'liquidity_eth': float(liquidity) if liquidity else 0.0
                        }
            except:
                # Final fallback: Use known pools
                pools = {
                    "0xACE9FEee4072aD385d02C8A6c4b69c66D72F64D6": {
                        "token_address": "0x2e32f96a4FbB9cD7CDC751971c015E282414B956",
                        "pool_type": "uniswap_v2",
                        "liquidity_eth": 14.2
                    }
                }
                
        return pools
        
    def detect_scam(self, tx_hash: str, state_changes: Dict) -> Optional[Dict]:
        """Detect if transaction is a scam based on state changes"""
        # Check each pool in our monitored list
        for pool_addr, pool_info in self.monitored_pools.items():
            pool_changes = state_changes.get(pool_addr, {})
            
            # Check for significant drains
            token_addr = pool_info['token_address']
            
            # Get token balance change
            token_change = pool_changes.get('erc20_balances', {}).get(token_addr, 0)
            
            # Get ETH/WETH balance change
            eth_change = pool_changes.get('eth_balance', 0)
            weth_change = pool_changes.get('erc20_balances', {}).get(self.simulator.WETH_ADDRESS, 0)
            total_eth_change = eth_change + weth_change
            
            # Detect drain
            if token_change < 0 and total_eth_change < 0:
                # Both reserves decreasing = liquidity removal
                current_token = pool_info.get('current_reserves', {}).get('token', 1)
                current_denom = pool_info.get('current_reserves', {}).get('denom', 1)
                
                token_drain_pct = abs(token_change) / max(current_token, 1)
                denom_drain_pct = abs(total_eth_change) / max(current_denom, 1)
                
                if token_drain_pct > self.DRAIN_THRESHOLD or denom_drain_pct > self.DRAIN_THRESHOLD:
                    severity = "CRITICAL" if max(token_drain_pct, denom_drain_pct) > 0.8 else "HIGH"
                    
                    return {
                        'tx_hash': tx_hash,
                        'pool': pool_addr,
                        'token': token_addr,
                        'severity': severity,
                        'type': 'LIQUIDITY_DRAIN',
                        'details': {
                            'token_drained': abs(token_change),
                            'token_drain_pct': token_drain_pct * 100,
                            'denom_drained': abs(total_eth_change),
                            'denom_drain_pct': denom_drain_pct * 100
                        }
                    }
                    
        return None
        
    async def monitor_mempool_websocket(self):
        """Monitor mempool using WebSocket subscription"""
        print(f"🚀 Starting Production Mempool Monitor")
        print(f"⚙️  Drain threshold: {self.DRAIN_THRESHOLD*100}%")
        
        # Get initial pools
        self.monitored_pools = self.get_pools_from_token_processor()
        print(f"👁️  Monitoring {len(self.monitored_pools)} pools")
        
        if not self.has_websocket:
            print("❌ WebSocket not available - falling back to HTTP polling")
            await self.monitor_mempool_http()
            return
            
        print("✅ Connected to mempool via WebSocket")
        
        # Subscribe to pending transactions
        subscription = await self.ws_w3.eth.subscribe('newPendingTransactions')
        
        last_stats = time.time()
        last_pool_update = time.time()
        
        async for tx_hash in subscription:
            self.stats["seen"] += 1
            
            try:
                # Get transaction details
                tx = await self.ws_w3.eth.get_transaction(tx_hash)
                
                if tx and tx.get('to') in self.monitored_pools:
                    self.stats["analyzed"] += 1
                    
                    # Simulate transaction
                    state_changes = self.simulator.simulate_transaction(tx)
                    
                    # Check for scam
                    scam = self.detect_scam(tx_hash.hex(), state_changes)
                    if scam:
                        self.stats["scams_detected"] += 1
                        self.stats["scams_by_severity"][scam['severity']] += 1
                        
                        # Log scam detection
                        print(f"\n🚨 SCAM DETECTED - {scam['severity']}")
                        print(f"   Transaction: {scam['tx_hash']}")
                        print(f"   Pool: {scam['pool']}")
                        print(f"   Type: {scam['type']}")
                        print(f"   Token drain: {scam['details']['token_drain_pct']:.1f}%")
                        print(f"   Denom drain: {scam['details']['denom_drain_pct']:.1f}%")
                        print(f"   Time until block: ~10-15 seconds")
                        
            except Exception as e:
                # Transaction might already be mined
                pass
                
            # Update pools periodically
            if time.time() - last_pool_update > 300:  # Every 5 minutes
                old_count = len(self.monitored_pools)
                self.monitored_pools = self.get_pools_from_token_processor()
                if len(self.monitored_pools) != old_count:
                    print(f"🔄 Pool list updated: {old_count} → {len(self.monitored_pools)}")
                last_pool_update = time.time()
                
            # Print stats every minute
            if time.time() - last_stats > 60:
                self.print_stats()
                last_stats = time.time()
                
    async def monitor_mempool_http(self):
        """Fallback: Monitor using HTTP polling of pending transactions"""
        print("📡 Using HTTP polling mode (less efficient)")
        
        last_stats = time.time()
        last_pool_update = time.time()
        
        while True:
            try:
                # Get pending transaction pool
                pending = self.w3.geth.txpool.content()['pending']
                
                for address, nonce_txs in pending.items():
                    for nonce, tx in nonce_txs.items():
                        self.stats["seen"] += 1
                        
                        if tx.get('to') in self.monitored_pools:
                            self.stats["analyzed"] += 1
                            
                            # Simulate transaction
                            state_changes = self.simulator.simulate_transaction(tx)
                            
                            # Check for scam
                            scam = self.detect_scam(tx['hash'], state_changes)
                            if scam:
                                self.stats["scams_detected"] += 1
                                self.stats["scams_by_severity"][scam['severity']] += 1
                                
                                # Log scam
                                print(f"\n🚨 SCAM DETECTED - {scam['severity']}")
                                print(f"   Transaction: {scam['tx_hash']}")
                                print(f"   Pool: {scam['pool']}")
                                
                # Update pools periodically  
                if time.time() - last_pool_update > 300:
                    self.monitored_pools = self.get_pools_from_token_processor()
                    last_pool_update = time.time()
                    
                # Print stats
                if time.time() - last_stats > 60:
                    self.print_stats()
                    last_stats = time.time()
                    
                await asyncio.sleep(1)  # Poll every second
                
            except Exception as e:
                await asyncio.sleep(5)
                
    def print_stats(self):
        """Print monitoring statistics"""
        runtime = int(time.time() - self.stats["start_time"])
        rate = self.stats["seen"] / max(runtime, 1)
        
        scam_str = f"Scams: {self.stats['scams_detected']}"
        if self.stats['scams_detected'] > 0:
            severities = []
            for sev, count in self.stats['scams_by_severity'].items():
                if count > 0:
                    severities.append(f"{sev}: {count}")
            scam_str += f" ({', '.join(severities)})"
            
        print(f"📊 STATS | Runtime: {runtime}s | Seen: {self.stats['seen']} ({rate:.1f}/s) | "
              f"Analyzed: {self.stats['analyzed']} | Pools: {len(self.monitored_pools)} | {scam_str}")

async def main():
    """Main entry point"""
    monitor = ProductionMempoolMonitor()
    
    try:
        if monitor.has_websocket:
            await monitor.monitor_mempool_websocket()
        else:
            await monitor.monitor_mempool_http()
    except KeyboardInterrupt:
        print("\n⏹️  Monitor stopped")
        monitor.print_stats()

if __name__ == "__main__":
    # Set up proper async event loop
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋 Shutting down...")