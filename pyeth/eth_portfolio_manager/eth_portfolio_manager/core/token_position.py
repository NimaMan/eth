"""
Token Position Aggregate Module

Objective:
----------
This module defines an aggregate class that centralizes all functionality related to a token'sposition across both backtesting and live trading processes. The aggregate encapsulates both thestatic metadata and the dynamic time-series snapshots pertaining to a token position, along with all domain logic (e.g., state transitions, performance computations, and serialization). This design ensures that token position data can be written to and read from the database in a consistent and JSON-friendly manner, facilitating analysis on the frontend via API routes and supporting real-time decision making in live trading.

Key Concepts:
-------------
1. Static Data:
   - Represents one-time or infrequently changed information about the token position.
   - Examples include:
       • Token address, symbol, and currency.
       • Entry/exit blocks and timestamps.
       • Purchase value and entry/exit price ratios.
       • Transaction fees.
   - Maintained in an instance of TokenPositionStaticData.

2. Dynamic Data (Snapshots):
   - Represents evolving metrics that change over time (typically per block or update iteration).
   - Examples include:
       • Current price (XPrice), current value.
       • Realized and unrealized profit.
       • Token age (in blocks/hours).
       • Latest block number and update timestamp.
       • Current state (e.g., INIT, BUY_SUBMITTED, BUY_CONFIRMED, SELL_SUBMITTED, SELL_CONFIRMED, SCAMMED).
   - Each update is stored as an instance of TokenPositionDynamicSnapshot.
   - Snapshots allow both historical time-series analysis (in backtesting) and real-time tracking (in live trading).

3. Domain Logic and Functionalities:
   - The aggregate class (TokenPosition) binds the static metadata and the list of dynamic snapshots.
   - It encapsulates all business logic related to token positions, such as:
       • Adding and retrieving snapshots.
       • Updating state based on live token data (price updates, block number, age, etc.). This is the update in the token position becuase of the addition of a new transaction and a block. 
       • Updating Scam Position. A scmaed position has a value of 0 and a scam probability of 1 and scam reason and label. 
       • Serializing the entire token position data structure to a dictionary for database persistence,
         API responses, and both offline and real-time analysis.
    
4. Integration with Persistence and Frontend:
   - The aggregate's `to_full_dict()` method aggregates static, dynamic, and computed fields into a
     single dictionary. This output is used:
         • By the persistence layer to store token positions in the database.
         • By API routes (e.g., via Flask's jsonify) to feed data to the frontend for both backtesting analysis and live monitoring.
   - All timestamps, enums, and numeric values are handled to ensure full JSON serialization compatibility,
     which is critical for both real-time and analytical use cases.

Usage Guidelines:
-----------------
- **Initialization/Creation:**  
  The token position can be created in two ways:
    1. **From a Token Update:**  
       Call `TokenPosition.create_from_token(token)`. This factory method extracts the necessary static data
       from a live or historical token update and initializes a new aggregate.
    2. **From a Persisted DB Record:**  
       Call `TokenPosition.from_dict(data)` with a dictionary (typically created via `to_full_dict()`).
  
- **Update Flow:**  
  1. Use `update_from_token_data(live_token)` to capture the latest dynamic metrics from token updates.
  2. Use `update_scammed_position(live_token)` to flag a position as compromised.
  
- **Serialization:**  
  Use `to_full_dict()` to generate a complete representation of the token position,
  suitable for both database insertion and JSON responses for the frontend.
"""

from typing import Optional, List, Tuple, Dict, Any
from eth_token.live_erc20_token.live_token import LiveERC20Token
from eth_token.live_erc20_token.data.live_token_data import TokenStatusEnum
from eth_portfolio_manager.core.data_models import TokenPositionStaticData, TokenPositionDynamicSnapshot, TokenPositionState


class TokenPosition:
    """
    TokenPosition Aggregate

    This class aggregates the static and dynamic data for a token's position and encapsulates
    the domain logic required for both backtesting and live trading. It coordinates state transitions,
    performance calculations, and data serialization.

    Functionalities:
    ----------------
    - Creation of a new position:
        • From a token update via the factory method `create_from_token`.
        • From a persisted data record via `from_dict`.
    - Adding new dynamic snapshots to track evolving metrics over time.
    - Updating token position with live token data and trading signals.
    - Handling special states (e.g., marking a position as SCAMMED).
    - Computing performance metrics (ROI, total return, etc.).
    - Serializing the complete token position into a JSON-friendly dictionary for persistence and API responses.
    """

    def __init__(self, static_data: TokenPositionStaticData, dynamic_history: Optional[List[TokenPositionDynamicSnapshot]] = None):
        self.static_data: TokenPositionStaticData = static_data
        self.dynamic_history: List[TokenPositionDynamicSnapshot] = dynamic_history if dynamic_history is not None else []
        self.add_entry_snapshot()
    
    @classmethod
    def create_from_token(cls, live_token: LiveERC20Token) -> "TokenPosition":
        """
        Factory method to create a new TokenPosition aggregate from a token update.

        This method extracts the necessary static data from the token update (whether in backtesting or live trading)
        and initializes the aggregate with a new TokenPositionStaticData instance.
        """
        static_data = TokenPositionStaticData(
            token_address=live_token.token_data.contract_address,
            symbol=live_token.token_data.symbol,
            currency=None,
            pool_address=None,
            pool_type=None,
            creation_block=live_token.token_data.creation_block,
            creation_timestamp=live_token.token_data.creation_timestamp,
            trading_enabled_block=live_token.token_data.trading_enabled_block,
            trading_enabled_timestamp=live_token.token_data.trading_enabled_timestamp,
            purchase_value=0.0,  # Initially zero, updated on buy signal.
            entry_price_ratio=None, # Initially zero, updated on buy signal.
            exit_price_ratio=None, # Initially zero, updated on sell signal.
            entry_block=live_token.token_data.creation_block, # update when the first buy signal is received
            exit_block=0, # update when the first sell signal is received
            entry_timestamp=live_token.token_data.creation_timestamp, # update when the first buy signal is received
            exit_timestamp=0, # update when the first sell signal is received
            entry_txn_fee=0.0, # update when the first buy signal is received
            exit_txn_fee=0.0 # update when the first sell signal is received
        )
        return cls(static_data)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TokenPosition":
        """
        Factory method to re-create a TokenPosition aggregate from a persisted dictionary record,
        typically one produced by the `to_full_dict()` method.

        This allows us to instantiate a TokenPosition from data stored in the database and then use it
        for API responses or further processing.
        """
        static_data = TokenPositionStaticData(**data["static"])
        dynamic_history = []
        for snap_data in data.get("dynamic_history", []):
            dynamic_history.append(TokenPositionDynamicSnapshot(**snap_data))
        return cls(static_data, dynamic_history=dynamic_history)

    def add_snapshot(self, snapshot: TokenPositionDynamicSnapshot) -> None:
        """
        Appends a new dynamic snapshot to the token position history.
        
        :param snapshot: TokenPositionDynamicSnapshot instance containing the latest metrics.
        """
        self.dynamic_history.append(snapshot)
    
    def add_entry_snapshot(self) -> None:
        """
        Adds an entry snapshot to the token position history.
        """
        self.add_snapshot(TokenPositionDynamicSnapshot())

    @property   
    def latest_snapshot(self) -> Optional[TokenPositionDynamicSnapshot]:
        """
        Returns the most recent dynamic snapshot.
        
        :return: The last recorded TokenPositionDynamicSnapshot, or None if no snapshots exist.
        """
        if self.dynamic_history:
            return self.dynamic_history[-1]
        return None
    
    def update_from_token_data(self, live_token: LiveERC20Token) -> None:
        """
        Updates the token position with live token data from the new block. It:
        1. Creates a new snapshot with updated metrics
        2. Updates token age, block numbers, timestamps
        3. Updates price data and calculations
        4. Updates scam-related metrics
        5. Updates position state based on the new block data
        """
        if live_token.token_data.token_status == TokenStatusEnum.INACTIVE_SCAM:
            self.update_scammed_position(live_token)
            return
        
        if live_token.token_data.trading_enabled_block:
            self.static_data.trading_enabled_block = live_token.token_data.trading_enabled_block
            self.static_data.trading_enabled_timestamp = live_token.token_data.trading_enabled_timestamp
            self.static_data.pool_address = tuple(live_token.token_data.pool_addresses)[0]
            self.static_data.pool_type = live_token.token_data.pool_info[self.static_data.pool_address]['pool_type']
            self.static_data.currency = live_token.token_data.pool_info[self.static_data.pool_address]['denom_currency']

        current_price_ratio = live_token.token_data.latest_pools_price_ratio.get(self.static_data.pool_address, 0)
        roi = 0
        current_value = 0
        unrealized_profit = 0
        if self.latest_snapshot.has_active_position:
            if self.static_data.entry_price_ratio:
                x_value = current_price_ratio / self.static_data.entry_price_ratio 
                roi = x_value - 1
                current_value = self.static_data.purchase_value * x_value
                unrealized_profit = current_value - self.static_data.purchase_value
        
        # Create new snapshot with updated data
        new_snapshot = TokenPositionDynamicSnapshot(
            current_price_ratio=current_price_ratio,
            reserve=live_token.token_data.get_pool_reserve(self.static_data.pool_address),
            roi=roi,
            current_value=current_value,
            realized_profit=self.latest_snapshot.realized_profit,
            unrealized_profit=unrealized_profit,
            quantity=self.latest_snapshot.quantity if self.latest_snapshot else 0,
            token_age_blocks=live_token.token_trading_age_blocks,
            token_age_hours=live_token.token_trading_age_hours,
            block_number=live_token.token_data.latest_block_number,
            timestamp=live_token.token_data.latest_block_timestamp,
            has_active_position=self.latest_snapshot.has_active_position if self.latest_snapshot else False,
            position_state=self.latest_snapshot.position_state if self.latest_snapshot else TokenPositionState.INIT,
            scam_probability=live_token.latest_token_assessment.get('scam_probability'),
            scam_reason=live_token.latest_token_assessment.get('scam_reason'),
            num_greys=live_token.latest_token_assessment.get('num_greys'),
            num_greens=live_token.latest_token_assessment.get('num_greens'),
            num_bribers=live_token.token_data.num_bribes,
            token_bribe_amount=live_token.token_data.total_bribe_amount
        )
        
        self.add_snapshot(new_snapshot)
    
    def update_scammed_position(self, live_token: LiveERC20Token) -> None:
        """
        Marks the token position as SCAMMED and updates relevant metrics.        
        """
        current_snapshot = self.latest_snapshot
        if current_snapshot:
            new_snapshot = TokenPositionDynamicSnapshot(
                current_price_ratio=0,
                roi=0,
                current_value=0,
                realized_profit=-self.static_data.purchase_value,
                unrealized_profit=0,
                quantity=current_snapshot.quantity,
                token_age_blocks=live_token.token_trading_age_blocks,
                token_age_hours=live_token.token_trading_age_hours,
                block_number=live_token.token_data.latest_block_number,
                timestamp=live_token.token_data.latest_block_timestamp,
                has_active_position=True,
                position_state=TokenPositionState.SCAMMED,
                scam_probability=1.0,
                scam_reason=live_token.latest_token_assessment.get('scam_reason'),
                num_greys=live_token.latest_token_assessment.get('num_greys'),
                num_greens=live_token.latest_token_assessment.get('num_greens'),
                num_bribers=live_token.token_data.num_bribes,
                token_bribe_amount=live_token.token_data.total_bribe_amount
            )
            self.add_snapshot(new_snapshot)
  
    def to_full_dict(self) -> Dict[str, Any]:
        """
        Serializes the complete token position data into a dictionary.
        
        The returned dictionary includes:
          - "static": Static token data.
          - "dynamic_history": List of all dynamic snapshots.
          - "performance": Computed performance metrics.
          
        This output is designed to be consumed by:
          - The BacktestResultsWriter for persistence.
          - Flask endpoints (via jsonify) to feed data to the frontend.
          
        :return: A complete dictionary representation of the token position.
        """
        return {
            "static": self.static_data.to_dict(),
            "dynamic_history": [snap.to_dict() for snap in self.dynamic_history],
        }

   