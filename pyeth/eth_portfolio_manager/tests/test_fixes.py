#!/usr/bin/env python3
"""Test the fixes for BacktestStrategyEngine and TokenPosition issues"""

import sys
sys.path.insert(0, '/home/nima/code/crypto/py/eth_portfolio_manager')

from eth_portfolio_manager.backtesting.backtest_strategy_engine import BacktestStrategyEngine
from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_portfolio_manager.core.token_position import TokenPosition
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision, TokenPositionState
from eth_token.erc20_token.erc20_token import ERC20Token
from typing import Optional

# Create a mock strategy to test BacktestStrategyEngine
class MockStrategy(BaseStrategy):
    def analyze_token(self, live_token: ERC20Token, token_position: TokenPosition) -> Optional[TradeSignal]:
        return None
    
    def handle_init_state(self, live_token: ERC20Token, token_position: TokenPosition) -> Optional[TradeSignal]:
        return None
    
    def handle_buy_submitted_state(self, live_token: ERC20Token, token_position: TokenPosition) -> Optional[TradeSignal]:
        return None
    
    def handle_buy_confirmed_state(self, live_token: ERC20Token, token_position: TokenPosition) -> Optional[TradeSignal]:
        return None
    
    def handle_sell_submitted_state(self, live_token: ERC20Token, token_position: TokenPosition) -> Optional[TradeSignal]:
        return None

def test_backtest_strategy_engine_has_strategy_name():
    """Test that BacktestStrategyEngine exposes strategy_name property"""
    print("Testing BacktestStrategyEngine.strategy_name property...")
    
    strategy = MockStrategy()
    engine = BacktestStrategyEngine(strategy)
    
    # Test that strategy_name is accessible
    try:
        strategy_name = engine.strategy_name
        print(f"✓ Successfully accessed strategy_name: {strategy_name}")
        assert strategy_name == "MockStrategy", f"Expected 'MockStrategy', got '{strategy_name}'"
        print("✓ strategy_name matches expected value")
    except AttributeError as e:
        print(f"✗ Failed to access strategy_name: {e}")
        return False
    
    return True

def test_token_position_handles_empty_pool_addresses():
    """Test that TokenPosition handles tokens without pool addresses gracefully"""
    print("\nTesting TokenPosition with empty pool addresses...")
    
    # Create mock token data class
    class MockTokenData:
        def __init__(self):
            self.contract_address = "0x1234"
            self.symbol = "TEST"
            self.creation_block = 1000
            self.creation_timestamp = 1234567890
            self.trading_enabled_block = None
            self.trading_enabled_timestamp = None
            self.pool_addresses = ()  # Empty tuple
            self.pool_info = {}
            self.latest_pools_price_ratio = {}
            self.latest_block_number = 2000
            self.latest_block_timestamp = 1234567900
            self.token_status = None
            self.num_bribes = 0
            self.total_bribe_amount = 0
            
        def get_pool_reserve(self, pool_address):
            return 0
    
    class MockLiveToken:
        def __init__(self):
            self.token_data = MockTokenData()
            self.token_trading_age_blocks = 100
            self.token_trading_age_hours = 1.5
            self.latest_token_assessment = {
                'scam_probability': 0,
                'scam_reason': None,
                'num_greys': 0,
                'num_greens': 0
            }
    
    # Create token position
    try:
        mock_token = MockLiveToken()
        position = TokenPosition.create_from_token(mock_token)
        print("✓ Successfully created TokenPosition from token without pools")
        
        # Test updating position with empty pool data
        mock_token.token_data.trading_enabled_block = 1500
        mock_token.token_data.trading_enabled_timestamp = 1234567895
        
        position.update_from_token_data(mock_token)
        print("✓ Successfully updated TokenPosition with empty pool data")
        
        # Verify pool_address is still None
        assert position.static_data.pool_address is None, "pool_address should be None"
        print("✓ pool_address correctly remains None")
        
    except Exception as e:
        print(f"✗ Failed with error: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    return True

def test_token_position_handles_pool_addresses():
    """Test that TokenPosition handles tokens with pool addresses correctly"""
    print("\nTesting TokenPosition with valid pool addresses...")
    
    # Create mock token data class
    class MockTokenData:
        def __init__(self):
            self.contract_address = "0x1234"
            self.symbol = "TEST"
            self.creation_block = 1000
            self.creation_timestamp = 1234567890
            self.trading_enabled_block = 1500
            self.trading_enabled_timestamp = 1234567895
            self.pool_addresses = ("0xpool1", "0xpool2")
            self.pool_info = {
                "0xpool1": {
                    'pool_type': 'V2',
                    'denom_currency': 'ETH'
                }
            }
            self.latest_pools_price_ratio = {
                "0xpool1": 1.5
            }
            self.latest_block_number = 2000
            self.latest_block_timestamp = 1234567900
            self.token_status = None
            self.num_bribes = 0
            self.total_bribe_amount = 0
            
        def get_pool_reserve(self, pool_address):
            if pool_address == "0xpool1":
                return 1000000
            return 0
    
    class MockLiveToken:
        def __init__(self):
            self.token_data = MockTokenData()
            self.token_trading_age_blocks = 100
            self.token_trading_age_hours = 1.5
            self.latest_token_assessment = {
                'scam_probability': 0,
                'scam_reason': None,
                'num_greys': 0,
                'num_greens': 0
            }
    
    # Create token position
    try:
        mock_token = MockLiveToken()
        position = TokenPosition.create_from_token(mock_token)
        print("✓ Successfully created TokenPosition from token with pools")
        
        # Update position
        position.update_from_token_data(mock_token)
        print("✓ Successfully updated TokenPosition with pool data")
        
        # Verify pool data was set correctly
        assert position.static_data.pool_address == "0xpool1", f"Expected '0xpool1', got '{position.static_data.pool_address}'"
        assert position.static_data.pool_type == "V2", f"Expected 'V2', got '{position.static_data.pool_type}'"
        assert position.static_data.currency == "ETH", f"Expected 'ETH', got '{position.static_data.currency}'"
        print("✓ Pool data correctly set")
        
    except Exception as e:
        print(f"✗ Failed with error: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    return True

if __name__ == "__main__":
    print("Running tests for eth_portfolio_manager fixes...\n")
    
    all_passed = True
    
    # Run tests
    all_passed &= test_backtest_strategy_engine_has_strategy_name()
    all_passed &= test_token_position_handles_empty_pool_addresses()
    all_passed &= test_token_position_handles_pool_addresses()
    
    print("\n" + "="*50)
    if all_passed:
        print("✅ All tests passed!")
    else:
        print("❌ Some tests failed!")
    
    sys.exit(0 if all_passed else 1)