"""
Live Trading Position Writer

Writes live trading positions to live_trading_db.
Manages the lifecycle of positions from creation to exit.
"""

from typing import Optional, Dict, Any
from decimal import Decimal
from datetime import datetime
from uuid import UUID
import psycopg2
from psycopg2.extras import RealDictCursor
from web3 import Web3

from .pool_registration_helper import PoolRegistrationHelper


class LiveTradingPositionWriter:
    """Writes and updates live trading positions in live_trading_db."""
    
    def __init__(self, logger):
        # Connection to live_trading_db
        self.live_conn = psycopg2.connect(
            dbname="live_trading_db",
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        self.live_cur = self.live_conn.cursor(cursor_factory=RealDictCursor)
        
        # Connection to eth_db for pool lookups
        self.eth_conn = psycopg2.connect(
            dbname="eth_db",
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        
        # Initialize pool helper
        self.pool_helper = PoolRegistrationHelper(self.eth_conn, logger)
        self.logger = logger
    
    def create_position(self, wallet_id: int, token_address: str, 
                       pool_address: Optional[str], strategy_name: str,
                       entry_signal_id: Optional[UUID] = None,
                       quantity_eth: Optional[float] = None) -> Optional[int]:
        """
        Create a new position entry.
        
        Args:
            wallet_id: ID of the wallet
            token_address: Token contract address
            pool_address: Pool address (None for V4)
            strategy_name: Name of the strategy
            entry_signal_id: UUID of the entry signal
            quantity_eth: ETH amount for the position
            
        Returns:
            Position ID if successful, None otherwise
        """
        try:
            # Convert addresses to checksum
            token_address = Web3.to_checksum_address(token_address)
            if pool_address:
                pool_address = Web3.to_checksum_address(pool_address)
            
            # Register pool and get pool_id
            pool_id = self.pool_helper.ensure_pool_registered(
                token_address=token_address,
                pool_address=pool_address
            )
            
            if not pool_id:
                self.logger.error(f"Failed to register pool for {token_address}")
                return None
            
            # Check for existing active position
            self.live_cur.execute("""
                SELECT id FROM live_positions
                WHERE wallet_id = %s AND pool_id = %s AND is_active = true
            """, (wallet_id, pool_id))
            
            if self.live_cur.fetchone():
                self.logger.warning(
                    f"Active position already exists for wallet {wallet_id} "
                    f"in pool {pool_id}"
                )
                return None
            
            # Insert new position
            self.live_cur.execute("""
                INSERT INTO live_positions 
                (wallet_id, token_address, pool_id, position_state, 
                 strategy_name, entry_signal_id, quantity_eth, is_active)
                VALUES (%s, %s, %s, 'INIT', %s, %s, %s, true)
                RETURNING id
            """, (
                wallet_id, token_address, pool_id, strategy_name,
                str(entry_signal_id) if entry_signal_id else None,
                quantity_eth
            ))
            
            self.live_conn.commit()
            result = self.live_cur.fetchone()
            position_id = result['id']
            
            self.logger.info(
                f"Created position {position_id} for wallet {wallet_id} "
                f"token {token_address[:10]}... pool_id {pool_id}"
            )
            
            return position_id
            
        except Exception as e:
            self.logger.error(f"Failed to create position: {e}")
            self.live_conn.rollback()
            return None
    
    def update_position_state(self, position_id: int, new_state: str,
                            update_data: Optional[Dict[str, Any]] = None) -> bool:
        """
        Update position state and related data.
        
        Args:
            position_id: Position ID to update
            new_state: New state (BUY_SUBMITTED, BUY_CONFIRMED, etc.)
            update_data: Additional data to update (tx_hash, price, etc.)
            
        Returns:
            True if successful, False otherwise
        """
        try:
            # Base update query
            update_parts = ["position_state = %s", "updated_at = %s"]
            update_values = [new_state, datetime.utcnow()]
            
            # Add additional updates based on state
            if new_state == 'BUY_CONFIRMED' and update_data:
                if 'entry_tx_hash' in update_data:
                    update_parts.append("entry_tx_hash = %s")
                    update_values.append(update_data['entry_tx_hash'])
                if 'entry_block' in update_data:
                    update_parts.append("entry_block = %s")
                    update_values.append(update_data['entry_block'])
                if 'entry_price' in update_data:
                    update_parts.append("entry_price = %s")
                    update_values.append(Decimal(str(update_data['entry_price'])))
                if 'quantity_tokens' in update_data:
                    update_parts.append("quantity_tokens = %s")
                    update_values.append(Decimal(str(update_data['quantity_tokens'])))
                update_parts.append("entry_timestamp = %s")
                update_values.append(datetime.utcnow())
                
            elif new_state == 'SELL_CONFIRMED' and update_data:
                if 'exit_tx_hash' in update_data:
                    update_parts.append("exit_tx_hash = %s")
                    update_values.append(update_data['exit_tx_hash'])
                if 'exit_block' in update_data:
                    update_parts.append("exit_block = %s")
                    update_values.append(update_data['exit_block'])
                if 'exit_price' in update_data:
                    update_parts.append("exit_price = %s")
                    update_values.append(Decimal(str(update_data['exit_price'])))
                if 'realized_profit_eth' in update_data:
                    update_parts.append("realized_profit_eth = %s")
                    update_values.append(Decimal(str(update_data['realized_profit_eth'])))
                update_parts.append("exit_timestamp = %s")
                update_values.append(datetime.utcnow())
                update_parts.append("is_active = %s")
                update_values.append(False)
            
            # Build and execute query
            update_sql = f"""
                UPDATE live_positions 
                SET {', '.join(update_parts)}
                WHERE id = %s
            """
            update_values.append(position_id)
            
            self.live_cur.execute(update_sql, update_values)
            self.live_conn.commit()
            
            self.logger.info(f"Updated position {position_id} to state {new_state}")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to update position state: {e}")
            self.live_conn.rollback()
            return False
    
    def update_position_metrics(self, position_id: int, 
                              current_price: float,
                              current_value_eth: float) -> bool:
        """
        Update position metrics (price, unrealized profit, etc.).
        
        Args:
            position_id: Position ID to update
            current_price: Current token price in ETH
            current_value_eth: Current position value in ETH
            
        Returns:
            True if successful, False otherwise
        """
        try:
            # Get position entry data
            self.live_cur.execute("""
                SELECT quantity_eth, entry_price
                FROM live_positions
                WHERE id = %s
            """, (position_id,))
            
            result = self.live_cur.fetchone()
            if not result:
                return False
            
            quantity_eth = float(result['quantity_eth'] or 0)
            entry_price = float(result['entry_price'] or 0)
            
            # Calculate metrics
            unrealized_profit = current_value_eth - quantity_eth
            roi_percent = ((current_value_eth / quantity_eth) - 1) * 100 if quantity_eth > 0 else 0
            
            # Update position
            self.live_cur.execute("""
                UPDATE live_positions
                SET current_price = %s,
                    current_value_eth = %s,
                    unrealized_profit_eth = %s,
                    roi_percent = %s,
                    updated_at = %s
                WHERE id = %s
            """, (
                Decimal(str(current_price)),
                Decimal(str(current_value_eth)),
                Decimal(str(unrealized_profit)),
                Decimal(str(roi_percent)),
                datetime.utcnow(),
                position_id
            ))
            
            self.live_conn.commit()
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to update position metrics: {e}")
            self.live_conn.rollback()
            return False
    
    def get_active_positions(self, wallet_id: Optional[int] = None,
                           strategy_name: Optional[str] = None) -> list:
        """
        Get active positions with optional filtering.
        
        Args:
            wallet_id: Filter by wallet ID
            strategy_name: Filter by strategy name
            
        Returns:
            List of active positions
        """
        try:
            query_parts = ["SELECT * FROM live_positions WHERE is_active = true"]
            query_params = []
            
            if wallet_id:
                query_parts.append("AND wallet_id = %s")
                query_params.append(wallet_id)
            
            if strategy_name:
                query_parts.append("AND strategy_name = %s")
                query_params.append(strategy_name)
            
            query = " ".join(query_parts) + " ORDER BY created_at DESC"
            self.live_cur.execute(query, query_params)
            
            return self.live_cur.fetchall()
            
        except Exception as e:
            self.logger.error(f"Failed to get active positions: {e}")
            return []
    
    def create_position_snapshot(self, position_id: int, block_number: int,
                               snapshot_data: Dict[str, Any]) -> bool:
        """
        Create a position snapshot for historical tracking.
        
        Args:
            position_id: Position ID
            block_number: Current block number
            snapshot_data: Snapshot data including price, value, pool reserves
            
        Returns:
            True if successful, False otherwise
        """
        try:
            self.live_cur.execute("""
                INSERT INTO position_snapshots
                (position_id, block_number, timestamp, token_price,
                 position_value_eth, unrealized_profit_eth, roi_percent,
                 pool_eth_reserve, pool_token_reserve, gas_price_gwei)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
            """, (
                position_id,
                block_number,
                datetime.utcnow(),
                Decimal(str(snapshot_data.get('token_price', 0))),
                Decimal(str(snapshot_data.get('position_value_eth', 0))),
                Decimal(str(snapshot_data.get('unrealized_profit_eth', 0))),
                Decimal(str(snapshot_data.get('roi_percent', 0))),
                Decimal(str(snapshot_data.get('pool_eth_reserve', 0))),
                Decimal(str(snapshot_data.get('pool_token_reserve', 0))),
                Decimal(str(snapshot_data.get('gas_price_gwei', 0)))
            ))
            
            self.live_conn.commit()
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to create position snapshot: {e}")
            self.live_conn.rollback()
            return False
    
    def __del__(self):
        if hasattr(self, 'live_cur'):
            self.live_cur.close()
        if hasattr(self, 'live_conn'):
            self.live_conn.close()
        if hasattr(self, 'eth_conn'):
            self.eth_conn.close()