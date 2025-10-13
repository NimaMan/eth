"""
Wallet Tracker Strategy

This strategy tracks positions for a specific wallet address and makes trading
decisions based on wallet activity and token health.

Wallet: 0x9f056A8d8F9127837B4c8005FA91BCeE6072ba81
"""

from typing import Optional, Dict, Any
from dataclasses import dataclass

from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_portfolio_manager.core.token_position import TokenPosition
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision
from eth_portfolio_manager.core.data_models import TokenPositionState
from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.utils.logger import get_logger


@dataclass
class WalletTrackerConfig:
    """Configuration for the wallet tracker strategy"""
    wallet_address: str = "0x9f056A8d8F9127837B4c8005FA91BCeE6072ba81"
    position_size_eth: float = 0.01  # Default position size
    profit_target_x: float = 1.5  # Take profit at 50% gain
    stop_loss_x: float = 0.7  # Stop loss at 30% loss
    max_positions: int = 10  # Maximum concurrent positions
    track_wallet_holdings: bool = True  # Track if wallet already holds the token
    

class WalletTrackerStrategy(BaseStrategy):
    """
    Strategy that tracks a specific wallet's holdings and makes trading decisions.
    
    This strategy:
    1. Monitors tokens for trading opportunities
    2. Checks if the wallet already has a position in the token
    3. Buys tokens when conditions are met and wallet doesn't hold them
    4. Sells based on profit targets or stop loss
    5. Publishes wallet position information
    """
    
    def __init__(self, config: Optional[WalletTrackerConfig] = None):
        """Initialize the wallet tracker strategy with configuration."""
        self.config = config or WalletTrackerConfig()
        self.logger = get_logger(f"WalletTrackerStrategy-{self.config.wallet_address[:8]}")
        
        # Track active positions for this wallet
        self._active_positions: Dict[str, bool] = {}
        
    @property
    def strategy_name(self) -> str:
        """Return the name of this strategy."""
        return f"WalletTracker_{self.config.wallet_address[:8]}"
    
    @property
    def strategy_parameters(self) -> Dict[str, Any]:
        """Return the strategy parameters as a dictionary."""
        return {
            "wallet_address": self.config.wallet_address,
            "position_size_eth": self.config.position_size_eth,
            "profit_target_x": self.config.profit_target_x,
            "stop_loss_x": self.config.stop_loss_x,
            "max_positions": self.config.max_positions,
            "track_wallet_holdings": self.config.track_wallet_holdings
        }
    
    def has_wallet_position(self, token: ERC20Token) -> bool:
        """
        Check if the wallet already has a position in this token.
        
        This would typically check on-chain balance, but for now we track
        internally. Can be enhanced to check actual wallet balance.
        """
        return self._active_positions.get(token.contract_address, False)
    
    def analyze_token(self, token: ERC20Token, position: Optional[TokenPosition]) -> Optional[TradeSignal]:
        """
        Main entry point for token analysis. Routes to appropriate handler
        based on position state.
        """
        if position is None:
            return self.handle_init_state(token)
        
        state = position.latest_snapshot.position_state
        
        if state == TokenPositionState.INIT:
            return self.handle_init_state(token)
        elif state == TokenPositionState.BUY_SUBMITTED:
            return self.handle_buy_submitted_state(token, position)
        elif state == TokenPositionState.BUY_CONFIRMED:
            return self.handle_buy_confirmed_state(token, position)
        elif state == TokenPositionState.SELL_SUBMITTED:
            return self.handle_sell_submitted_state(token, position)
        elif state == TokenPositionState.SELL_CONFIRMED:
            # Position closed, remove from tracking
            self._active_positions[token.contract_address] = False
            return None
        elif state == TokenPositionState.SCAMMED:
            # Token scammed, remove from tracking
            self._active_positions[token.contract_address] = False
            return None
        
        return None
    
    def handle_init_state(self, token: ERC20Token) -> Optional[TradeSignal]:
        """
        Decide whether to buy a token when no position exists.
        
        Buy criteria:
        1. Token must have trading enabled
        2. Token must not be marked as scam
        3. Wallet must not already hold the token (if tracking enabled)
        4. Must have room for more positions
        """
        # Skip if wallet already has position and tracking is enabled
        if self.config.track_wallet_holdings and self.has_wallet_position(token):
            self.logger.debug(f"Wallet already holds token {token.contract_address}")
            return None
        
        # Check if we have room for more positions
        active_count = sum(1 for active in self._active_positions.values() if active)
        if active_count >= self.config.max_positions:
            self.logger.debug(f"Max positions reached: {active_count}/{self.config.max_positions}")
            return None
        
        # Check token status
        token_status = getattr(token.token_data, 'token_status', None)
        if token_status != 'TRADING_ENABLED':
            return None
        
        # Check for scam
        if hasattr(token, 'is_scam') and token.is_scam:
            self.logger.info(f"Skipping scam token {token.contract_address}: {token.scam_reason}")
            return None
        
        # Additional health checks
        latest_assessment = getattr(token, 'latest_token_assessment', {})
        if latest_assessment.get('is_scam', False):
            return None
        
        # Token looks good, submit buy
        self.logger.info(f"Submitting buy for token {token.contract_address}")
        self._active_positions[token.contract_address] = True
        
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.SUBMIT_BUY,
            quantity=self.config.position_size_eth,
            strategy_name=self.strategy_name
        )
    
    def handle_buy_submitted_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """Confirm the buy transaction was successful."""
        # Always confirm buy in next iteration
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.CONFIRM_BUY,
            quantity=position.latest_snapshot.quantity,
            strategy_name=self.strategy_name
        )
    
    def handle_buy_confirmed_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """
        Monitor the position and decide when to sell.
        
        Sell criteria:
        1. Profit target reached (default 50% gain)
        2. Stop loss triggered (default 30% loss)
        3. Token marked as scam
        """
        snapshot = position.latest_snapshot
        
        # Check if token became a scam
        if hasattr(token, 'is_scam') and token.is_scam:
            self.logger.warning(f"Token {token.contract_address} marked as scam, selling position")
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_SELL,
                quantity=snapshot.quantity,
                strategy_name=self.strategy_name
            )
        
        # Check profit target
        if snapshot.roi >= self.config.profit_target_x:
            self.logger.info(
                f"Profit target reached for {token.contract_address}: "
                f"ROI={snapshot.roi:.2f}x >= {self.config.profit_target_x}x"
            )
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_SELL,
                quantity=snapshot.quantity,
                strategy_name=self.strategy_name
            )
        
        # Check stop loss
        if snapshot.roi <= self.config.stop_loss_x:
            self.logger.info(
                f"Stop loss triggered for {token.contract_address}: "
                f"ROI={snapshot.roi:.2f}x <= {self.config.stop_loss_x}x"
            )
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_SELL,
                quantity=snapshot.quantity,
                strategy_name=self.strategy_name
            )
        
        # Hold position
        return None
    
    def handle_sell_submitted_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """Confirm the sell transaction was successful."""
        # Always confirm sell in next iteration
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.CONFIRM_SELL,
            quantity=position.latest_snapshot.quantity,
            strategy_name=self.strategy_name
        )
    
    def get_position_info(self) -> Dict[str, Any]:
        """
        Get current position information for publishing.
        
        Returns information about:
        - Wallet address
        - Active positions
        - Total position count
        - Strategy parameters
        """
        return {
            "wallet_address": self.config.wallet_address,
            "active_positions": dict(self._active_positions),
            "active_count": sum(1 for active in self._active_positions.values() if active),
            "max_positions": self.config.max_positions,
            "strategy_name": self.strategy_name,
            "parameters": self.strategy_parameters
        }