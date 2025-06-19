#!/usr/bin/env python3
"""
Example Execution Bot Integration
Shows how to receive alerts and execute protective trades
"""

import zmq
import json
import time
from web3 import Web3
from datetime import datetime

class ScamProtectionBot:
    def __init__(self, zmq_endpoint="tcp://localhost:5559", web3_url="http://localhost:8545"):
        # ZMQ setup
        self.context = zmq.Context()
        self.subscriber = self.context.socket(zmq.SUB)
        self.subscriber.connect(zmq_endpoint)
        self.subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
        
        # Web3 setup (for executing transactions)
        self.w3 = Web3(Web3.HTTPProvider(web3_url))
        
        # Bot configuration
        self.min_eth_threshold = 0.1  # Minimum ETH to protect
        self.max_gas_price = 100  # Max gas price in gwei
        self.confidence_threshold = 0.8  # Minimum confidence to act
        
        # Tracking
        self.alerts_received = 0
        self.actions_taken = 0
        
        print(f"🤖 Scam Protection Bot initialized")
        print(f"📡 Listening for alerts on {zmq_endpoint}")
        print(f"🌐 Connected to Web3: {self.w3.is_connected()}")
        
    def run(self):
        """Main bot loop"""
        print("\n⚡ Bot is running... (Press Ctrl+C to stop)\n")
        
        try:
            while True:
                # Receive alert (blocking)
                message = self.subscriber.recv_string()
                alert = json.loads(message)
                self.alerts_received += 1
                
                # Process alert
                self.process_alert(alert)
                
        except KeyboardInterrupt:
            print(f"\n\n🛑 Bot stopped")
            print(f"📊 Stats: {self.alerts_received} alerts received, {self.actions_taken} actions taken")
        finally:
            self.cleanup()
            
    def process_alert(self, alert):
        """Process incoming alert and decide on action"""
        timestamp = datetime.fromtimestamp(alert['timestamp'] / 1000)
        
        print(f"\n{'='*60}")
        print(f"🚨 Alert #{self.alerts_received} received at {timestamp}")
        print(f"Type: {alert['event_type']} | Severity: {alert['severity']}")
        print(f"Pool: {alert['pool_address']}")
        print(f"Token: {alert['token_address']}")
        print(f"ETH Impact: {alert['eth_change_amount']:.4f} ({alert['eth_change_percent']:.1f}%)")
        print(f"Confidence: {alert['confidence_score']:.2f}")
        
        # Decision logic
        if alert['event_type'] == 'ScamAlert':
            self.handle_scam_alert(alert)
        elif alert['event_type'] == 'LiquidityWarning':
            self.handle_liquidity_warning(alert)
        elif alert['event_type'] == 'LargeTrade':
            self.handle_large_trade(alert)
        else:
            print(f"ℹ️  Event type {alert['event_type']} - monitoring only")
            
    def handle_scam_alert(self, alert):
        """Handle critical scam alerts"""
        print(f"\n🚨 SCAM ALERT HANDLER")
        
        # Check if we should act
        if alert['confidence_score'] < self.confidence_threshold:
            print(f"❌ Confidence too low: {alert['confidence_score']} < {self.confidence_threshold}")
            return
            
        if abs(alert['eth_change_amount']) < self.min_eth_threshold:
            print(f"❌ ETH amount too small: {abs(alert['eth_change_amount']):.4f} < {self.min_eth_threshold}")
            return
            
        # Simulate checking if we hold this token
        if not self.check_token_holdings(alert['token_address']):
            print(f"✅ Not holding token - no action needed")
            return
            
        # Calculate protection strategy
        strategy = self.calculate_protection_strategy(alert)
        
        if strategy['action'] == 'SELL':
            print(f"\n💰 PROTECTION STRATEGY:")
            print(f"   Action: {strategy['action']}")
            print(f"   Amount: {strategy['amount']} tokens")
            print(f"   Route: {strategy['route']}")
            print(f"   Max Slippage: {strategy['max_slippage']}%")
            print(f"   Estimated Gas: {strategy['gas_estimate']} gwei")
            
            # In production, execute the trade here
            if self.simulate_execution(strategy):
                self.actions_taken += 1
                print(f"✅ Protection executed successfully!")
            else:
                print(f"❌ Protection execution failed!")
                
    def handle_liquidity_warning(self, alert):
        """Handle liquidity warnings"""
        print(f"\n⚠️  LIQUIDITY WARNING HANDLER")
        
        # Less urgent - maybe set up monitoring
        if abs(alert['eth_change_percent']) > 30:
            print(f"📍 Adding pool to high-risk watchlist")
            print(f"📊 Will monitor for follow-up activity")
            # In production: Add to monitoring system
            
    def handle_large_trade(self, alert):
        """Handle large trade alerts for arbitrage"""
        print(f"\n💹 LARGE TRADE HANDLER")
        
        # Check for arbitrage opportunity
        if abs(alert['price_impact_percent']) > 5:
            print(f"🎯 Potential arbitrage opportunity detected!")
            print(f"   Price impact: {alert['price_impact_percent']:.2f}%")
            # In production: Calculate and execute arbitrage
            
    def check_token_holdings(self, token_address):
        """Check if we hold this token (simulated)"""
        # In production: Query actual wallet/contract balances
        # For demo, randomly return True 30% of time
        import random
        return random.random() < 0.3
        
    def calculate_protection_strategy(self, alert):
        """Calculate optimal protection strategy"""
        return {
            'action': 'SELL',
            'amount': 'ALL',  # Sell all holdings
            'route': 'UniswapV2' if alert['pool_version'] == 'V3' else 'UniswapV3',  # Use different pool
            'max_slippage': 50,  # Accept high slippage in emergency
            'gas_estimate': 50,  # gwei
            'deadline': int(time.time()) + 120  # 2 minute deadline
        }
        
    def simulate_execution(self, strategy):
        """Simulate trade execution"""
        print(f"\n🔄 Simulating trade execution...")
        time.sleep(0.5)  # Simulate execution time
        
        # In production: 
        # 1. Build transaction
        # 2. Estimate gas
        # 3. Sign and send
        # 4. Wait for confirmation
        
        # For demo, always succeed
        return True
        
    def cleanup(self):
        """Clean up resources"""
        self.subscriber.close()
        self.context.term()


class ArbitrageBot:
    """Example arbitrage bot that acts on price impact alerts"""
    
    def __init__(self, zmq_endpoint="tcp://localhost:5559"):
        self.context = zmq.Context()
        self.subscriber = self.context.socket(zmq.SUB)
        self.subscriber.connect(zmq_endpoint)
        self.subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
        
        self.min_profit_eth = 0.01  # Minimum profit to execute
        self.opportunities_found = 0
        
    def calculate_arbitrage(self, alert):
        """Calculate arbitrage profitability"""
        if alert['event_type'] not in ['LargeTrade', 'PriceImpact']:
            return None
            
        # Simplified calculation
        price_impact = abs(alert['price_impact_percent'])
        pool_size = alert['current_eth_reserve']
        
        # Estimate profit (very simplified)
        potential_profit = pool_size * (price_impact / 100) * 0.5  # 50% capture
        gas_cost = 0.01  # Estimated gas in ETH
        
        net_profit = potential_profit - gas_cost
        
        if net_profit > self.min_profit_eth:
            return {
                'profitable': True,
                'gross_profit': potential_profit,
                'gas_cost': gas_cost,
                'net_profit': net_profit,
                'confidence': min(price_impact / 10, 1.0)  # Higher impact = higher confidence
            }
        return None


def main():
    """Run example bot"""
    import sys
    
    print("🤖 Execution Bot Integration Example")
    print("Choose bot type:")
    print("1. Scam Protection Bot")
    print("2. Arbitrage Bot")
    
    choice = input("\nEnter choice (1 or 2): ").strip()
    
    if choice == "1":
        bot = ScamProtectionBot()
        bot.run()
    elif choice == "2":
        print("\n🚧 Arbitrage bot example not fully implemented")
        print("See ScamProtectionBot for pattern to follow")
    else:
        print("Invalid choice")
        sys.exit(1)


if __name__ == "__main__":
    main()