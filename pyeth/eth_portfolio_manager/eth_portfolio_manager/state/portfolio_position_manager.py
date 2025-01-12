"""
Portfolio Position Manager

Objective:
---------
1. Manage portfolio-wide position tracking
2. Coordinate position updates across multiple tokens
3. Maintain portfolio state persistence
4. Calculate portfolio-wide metrics

Position Management Flow:
----------------------
1. Token Position Creation:
   - New token detected by PortfolioManager
   - Creates TokenPositionData in INIT state
   - Adds to portfolio position tracking
   - Initializes Redis persistence

2. Position Updates:
   - Receives batch updates from PortfolioManager
   - Processes updates concurrently for efficiency
   - Updates individual positions via LiveTokenPositionManager
   - Persists changes to Redis

3. Portfolio Metrics:
   - Tracks total portfolio value
   - Calculates aggregate P&L
   - Monitors active position count
   - Updates metrics after position changes

4. State Persistence:
   - Maintains Redis connection
   - Atomic updates to prevent race conditions
   - Periodic state snapshots
   - Recovery from persistence layer

Event Handling:
-------------
1. New Token Detection:
   - Source: PortfolioManager token creation events
   - Handler: create_position()
   - Action: Initializes new position tracking
   - Updates: Redis state, portfolio metrics

2. Token Updates:
   - Source: PortfolioManager batch updates
   - Handler: update_token_positions()
   - Action: Concurrent position processing
   - Updates: Position states, Redis, metrics

3. Position State Changes:
   - Source: LiveTokenPositionManager
   - Handler: _process_single_token()
   - Action: Updates individual position states
   - Updates: Position data, portfolio metrics

4. Portfolio Metrics:
   - Source: Any position change
   - Handler: update_portfolio_metrics()
   - Action: Recalculates portfolio totals
   - Updates: Metrics in Redis

Data Flow:
---------
1. Token Creation:
   PortfolioManager -> create_position()
   -> Redis -> update_portfolio_metrics()

2. Position Updates:
   PortfolioManager -> update_token_positions()
   -> _process_single_token() -> LiveTokenPositionManager
   -> Redis -> update_portfolio_metrics()

3. Metrics Updates:
   Position changes -> update_portfolio_metrics()
   -> Redis -> PortfolioMetrics

4. State Persistence:
   Any state change -> PortfolioStateServer
   -> Redis -> Recovery on restart

Key Metrics Tracked:
------------------
1. Total Portfolio Value
2. Total Profit/Loss
3. Active Position Count
4. Last Update Timestamp
5. Individual Position States
6. Portfolio State History

Implementation Notes:
------------------
1. Concurrent position processing
2. Atomic Redis updates
3. Efficient state management
4. Error recovery handling
5. Metrics calculation optimization
"""

from typing import Dict, List, Optional, Tuple
import asyncio

from eth_portfolio_manager.core.data_models import TokenPositionData, TokenPositionState
from eth_portfolio_manager.state.live_token_position_manager import LiveTokenPositionManager
from eth_portfolio_manager.state.portfolio_state_server import PortfolioStateServer
from eth_portfolio_manager.state.portfolio_metrics_calculator import PortfolioMetricsCalculator
from eth_token_monitor.live_erc20_token.data.live_token_data import LiveTokenData
from eth_portfolio_manager.utils.logger import get_logger


class PortfolioPositionManager:
    def __init__(self, logger=None):
        self.logger = logger or get_logger(name="portfolio_manager")
        self.positions: Dict[str, TokenPositionData] = {}
        self.metrics_calculator = PortfolioMetricsCalculator()
        self.token_position_manager = None
       
    async def _process_single_token(self, token: LiveTokenData) -> Tuple[str, Optional[TokenPositionData]]:
        """Process updates for a single token"""
        try:
            # Create or get existing position
            token_address = token.contract_address
            if token_address not in self.positions:
                position = self.create_position(token)
                self.positions[token_address] = position
            else:
                position = self.positions[token_address]

            # Process token updates
            updated_position = await self.token_position_manager.process_token_updates(token, position)
            return token_address, updated_position
            
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing token {token_address} with strategy {self.token_position_manager.strategy_name}: {e}")
            return token_address, None

    def create_position(self, token: LiveTokenData) -> TokenPositionData:
        """Create a new position for a token"""
        return TokenPositionData(
            symbol=token.symbol,
            entry_Xprice=0,
            current_Xprice=0,
            Xprice=0,
            purchase_value=0,
            current_value=0,
            realized_profit=0,
            unrealized_profit=0,
            quantity=0,
            token_age_blocks=token.latest_block_number - token.creation_block,
            token_age_hours=0,
            last_updated_block=token.latest_block_number,
            last_updated_time=token.latest_block_timestamp,
            entry_block=0,
            has_active_position=False,
            position_state=TokenPositionState.INIT,
            token_address=token.contract_address
        )

    async def get_portfolio_metrics(self):
        """Calculate and update portfolio-wide metrics"""
        try:
            metrics = await self.metrics_calculator.update_metrics(self.positions)
            self.metrics = metrics
            return metrics
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error updating portfolio metrics: {e}")
            raise
            
    @property
    def strategy_name(self) -> str:
        """Return the name of the strategy"""
        return self.token_position_manager.strategy_name


class LivePortfolioPositionManager(PortfolioPositionManager):
    def __init__(self, logger=None):
        super().__init__(logger=logger)
        self.state_server = PortfolioStateServer(logger=self.logger)
        self.positions: Dict[str, TokenPositionData] = self.state_server.current_positions
        self.token_position_manager = LiveTokenPositionManager()
        
    async def initialize(self):
        """Initialize portfolio state from Redis"""
        try:
            await self.state_server.load_state()
            self.positions = self.state_server.current_positions
            await self.update_portfolio_metrics(self.positions)
            self.logger.info(f"Initialized portfolio with {len(self.positions)} positions")
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error initializing portfolio: {e}")
            raise e

    async def update_token_positions(self, updated_tokens: Dict[str, LiveTokenData]) -> Dict[str, TokenPositionData]:
        """
        Process multiple token updates concurrently and update portfolio state
        
        Args:
            updated_tokens: Dict mapping token addresses to their updated data
            
        Returns:
            Dict of updated position data
            
        Implementation:
        1. Group tokens into batches for efficient processing
        2. Process each batch concurrently using asyncio.gather
        3. Update Redis state atomically
        4. Update portfolio metrics
        """
        try:
            # Process token updates in parallel
            update_tasks = []
            for token_address, token in updated_tokens.items():
                task = self._process_single_token(token)
                update_tasks.append(task)
            
            # Wait for all updates to complete
            position_updates = await asyncio.gather(*update_tasks)
            
            # Combine updates into single dict
            updated_positions = {
                addr: pos for addr, pos in position_updates if pos is not None
            }
            
            # Batch update Redis state and metrics
            if updated_positions:
                await self.state_server.update_positions(updated_positions)
                await self.update_portfolio_metrics(updated_positions)
            return updated_positions
            
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error updating token positions: {e}")
            raise
    
    def get_position(self, token_address: str) -> Optional[TokenPositionData]:
        """Get position data for a specific token"""
        return self.positions.get(token_address)
   