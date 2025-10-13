from sqlalchemy import text, select
from sqlalchemy.exc import IntegrityError
import traceback
import time
from psycopg2.errors import ForeignKeyViolation
import eth_data.database.schema.eth_db_data_models as models
from eth_data.database.eth_db_conn import get_db_session_maker
from eth_data.database.writers.token_status_writer import TokenStatusWriter


class TokenPnLWriter:
    """
    Handles database operations for writing token PnL data.
    
    This class encapsulates all database-related functionality for persisting
    token performance metrics to the database.
    """
    
    def __init__(self, logger=None):
        """
        Initialize the TokenPnLWriter.
        
        Args:
            logger: Logger instance
        """
        self.logger = logger
        self.Session = get_db_session_maker(db='eth_db')
        # Delegate token creation/updates to dedicated writer
        self.token_status_writer = TokenStatusWriter(logger=logger)
    
    def _mark_creator_as_scammer(self, token):
        """Checks if a token is a scam and marks its creator's address label accordingly."""
        token_address = token.contract_address
        creator_address = token.token_data.creator_address
        is_scam = token.token_data.is_scam

        if not (is_scam and creator_address):
            return # Nothing to do if it's not a scam or no creator address

        if not self.Session:
            if self.logger: self.logger.error("Database session not initialized for scammer marking.")
            return

        with self.Session() as session:
            try:
                creator_address_id = self._get_or_create_address_id(session, creator_address)
                if creator_address_id:
                    update_stmt = text("""
                        UPDATE eth_db.addresses
                        SET cluster_label = :label
                        WHERE address_id = :id AND (cluster_label IS NULL OR cluster_label != :label)
                    """)
                    session.execute(update_stmt, {"label": "Scammer", "id": creator_address_id})
                    session.commit()
            except Exception as update_err:
                session.rollback()
                if self.logger:
                    self.logger.error(f"Failed to update cluster_label for scammer {creator_address} (token: {token_address}): {update_err}")
                    self.logger.error(traceback.format_exc())

    def write_token_pnl_to_db(self, token) -> bool:
        """
        Write PnL data for a token to the database and update creator label if it's a scam.
        
        Args:
            token: LiveTokenData instance
            
        Returns:
            bool: True if successful, False otherwise
        """
        if not self.Session:
            if self.logger: self.logger.error("Database session not initialized.")
            return False
            
        token_address = token.contract_address
        
        try:
            # --- Mark Creator as Scammer (Refactored Call) ---            
            self._mark_creator_as_scammer(token)
            if token.token_network is None:
                 return False
            token_network = token.token_network

            # Get user activity dataframe with PnL metrics
            user_activity_df = token_network.get_agg_user_activity_df()
            
            if user_activity_df is None or user_activity_df.empty:
                # Only log if there's an actual error, not just no activity
                pass
                # Still ensure the token exists in the DB even if no activity
                if hasattr(token, 'contract_address'): # Check if token object is valid enough
                     # DEBUG: Log token data fields
                     # self.logger.debug(f"TokenPnLWriter (no activity) - Token {token.contract_address}: creation_tx={token.token_data.creation_tx}, trading_enabled_tx={token.token_data.trading_enabled_tx}")
                     
                     token_db_data = {
                         "contract_address": token.contract_address,
                         "creator_address": token.token_data.creator_address,
                         "is_scam": token.token_data.is_scam,
                         "scam_label": token.token_data.scam_label,
                         "creation_tx": token.token_data.creation_tx
                     }
                     self._ensure_token_in_db(token_db_data)
                return True # Return True as the operation wasn't an error, just no data
                
            # Ensure token record exists in the database
            # DEBUG: Log token data fields
            # self.logger.debug(f"TokenPnLWriter (with activity) - Token {token.contract_address}: creation_tx={token.token_data.creation_tx}, trading_enabled_tx={token.token_data.trading_enabled_tx}")
            
            token_db_data = {
                "contract_address": token.contract_address,
                "creator_address": token.token_data.creator_address,
                "is_scam": token.token_data.is_scam,
                "scam_label": token.token_data.scam_label,
                "creation_tx": token.token_data.creation_tx
            }
            self._ensure_token_in_db(token_db_data)
            
            # Write user PnL data
            self._write_user_trade_pnl_data(user_activity_df, token_address)            
            return True
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error writing PnL data for token {token_address}: {str(e)}")
                self.logger.error(traceback.format_exc())
            return False
    
    def _ensure_token_in_db(self, token_data):
        """
        Ensure token exists in the database using provided dict.
        
        Note: This method only handles token data for PnL operations.
        For full token+pool persistence from live tracking, use 
        TokenStatusWriter.create_or_update_token_with_pools() directly.
        """
        try:
            # Delegate token creation/updates to the dedicated writer
            success = self.token_status_writer.create_or_update_token(token_data)
            if not success and self.logger:
                self.logger.error(f"Failed to ensure token {token_data['contract_address']} in database")
            return success
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error ensuring token in DB: {e}")
            return False
    
    def _get_or_create_address_id(self, session, address_str):
        """
        Get the address_id for the given address string, creating it if it doesn't exist.
        
        Args:
            session: SQLAlchemy session
            address_str: Ethereum address string
            
        Returns:
            int: The address_id
        """
        if not address_str:
            return None
            
        # Check if address exists
        result = session.execute(
            select(models.Address.address_id).where(models.Address.address == address_str)
        ).scalar_one_or_none()
        
        if result:
            return result
        
        # If not, insert it
        try:
            insert_result = session.execute(
                text("""
                INSERT INTO eth_db.addresses (address, is_contract) 
                VALUES (:address, false) 
                ON CONFLICT (address) DO UPDATE SET address=EXCLUDED.address
                RETURNING address_id
                """),
                {"address": address_str}
            )
            address_id = insert_result.scalar_one()
            return address_id
        except IntegrityError:
            # If we hit a race condition, try to get the ID again
            return session.execute(
                select(models.Address.address_id).where(models.Address.address == address_str)
            ).scalar_one()
    
    def _write_user_trade_pnl_data(self, user_activity_df, token_address):
        """Write user PnL data to the trades table"""
        with self.Session() as session:
            try:
                # Reset index to get address as a column
                if 'address' not in user_activity_df.columns and user_activity_df.index.name == 'address':
                    user_activity_df = user_activity_df.reset_index()
                
                # Process each user's PnL data
                for _, row in user_activity_df.iterrows():
                    address = row['address']
                    
                    # Get address_id for this user
                    address_id = self._get_or_create_address_id(session, address)
                    
                    # Check if trade record exists for this address_id, token, and currency
                    # Currently only supporting ETH as currency
                    currency = "ETH"
                    result = session.execute(
                        text("""
                        SELECT id FROM eth_db.trades 
                        WHERE address_id = :addr_id AND token_address = :token_addr AND currency = :currency
                        """),
                        {"addr_id": address_id, "token_addr": token_address, "currency": currency}
                    ).fetchone()
                    
                    if result:
                        # Update existing record
                        trade_id = result[0]
                        session.execute(
                            text("""
                            UPDATE eth_db.trades SET
                            entry_block = :entry_block,
                            latest_block = :latest_block,
                            total_denom_spent = :total_denom_spent,
                            total_denom_received = :total_denom_received,
                            denom_received_spent_ratio = :denom_received_spent_ratio,
                            realized_profit = :realized_profit,
                            unrealized_profit = :unrealized_profit,
                            num_buys = :num_buys,
                            num_sells = :num_sells,
                            total_gas_spent = :total_gas_spent,
                            token_balance = :token_balance,
                            denom_balance = :denom_balance,
                            last_updated = CURRENT_TIMESTAMP
                            WHERE id = :id
                            """),
                            {
                                "id": trade_id,
                                "entry_block": row.get('entry_block'),
                                "latest_block": row.get('latest_block'),
                                "total_denom_spent": row.get('total_denom_spent', 0),
                                "total_denom_received": row.get('total_denom_received', 0),
                                "denom_received_spent_ratio": row.get('denom_received_spent_ratio'),
                                "realized_profit": row.get('realized_profit', 0),
                                "unrealized_profit": row.get('unrealized_profit', 0),
                                "num_buys": row.get('num_buys', 0),
                                "num_sells": row.get('num_sells', 0),
                                "total_gas_spent": row.get('tx_fee', 0),  # Map tx_fee to total_gas_spent
                                "token_balance": row.get('agg_token_balance', 0),  # Map to new column name
                                "denom_balance": row.get('agg_denom_balance', 0),  # Map to new column name
                            }
                        )
                        
                    else:
                        # Insert new record
                        result = session.execute(
                            text("""
                            INSERT INTO eth_db.trades
                            (address_id, token_address, currency, entry_block, latest_block, 
                             total_denom_spent, total_denom_received, denom_received_spent_ratio, 
                             realized_profit, unrealized_profit, num_buys, num_sells,
                             total_gas_spent, token_balance, denom_balance)
                            VALUES
                            (:address_id, :token_address, :currency, :entry_block, :latest_block,
                             :total_denom_spent, :total_denom_received, :denom_received_spent_ratio,
                             :realized_profit, :unrealized_profit, :num_buys, :num_sells,
                             :total_gas_spent, :token_balance, :denom_balance)
                            RETURNING id
                            """),
                            {
                                "address_id": address_id,
                                "token_address": token_address,
                                "currency": currency,  # Use the currency variable defined above
                                "entry_block": row.get('entry_block'),
                                "latest_block": row.get('latest_block'),
                                "total_denom_spent": row.get('total_denom_spent', 0),
                                "total_denom_received": row.get('total_denom_received', 0),
                                "denom_received_spent_ratio": row.get('denom_received_spent_ratio'),
                                "realized_profit": row.get('realized_profit', 0),
                                "unrealized_profit": row.get('unrealized_profit', 0),
                                "num_buys": row.get('num_buys', 0),
                                "num_sells": row.get('num_sells', 0),
                                "total_gas_spent": row.get('tx_fee', 0),  # Map tx_fee to total_gas_spent
                                "token_balance": row.get('agg_token_balance', 0),  # Map to new column name
                                "denom_balance": row.get('agg_denom_balance', 0),  # Map to new column name
                            }
                        )
                        
                        trade_id = result.scalar_one()
                
                session.commit()
            except Exception as e:
                session.rollback()
                if self.logger:
                    self.logger.error(f"Error writing user PnL data: {str(e)}")
                    self.logger.error(traceback.format_exc())
                raise