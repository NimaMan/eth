"""
Token Status Writer - Handles token and pool lifecycle updates in eth_db

This module is responsible for:
1. Creating new token records in eth_db.tokens
2. Updating token status (scam detection, trading status, etc.)
3. Creating and updating pool records in eth_db.pools
4. Managing token-pool relationships and metadata persistence

Handles both tokens and pools since they are tightly coupled in the live tracking system.
"""

from sqlalchemy import text
from typing import Dict, Any, Optional
import logging
import time
from psycopg2.errors import ForeignKeyViolation
from eth_data.database.eth_db_conn import get_db_session_maker


class TokenStatusWriter:
    """
    Handles token and pool status persistence to eth_db.tokens and eth_db.pools tables.
    
    Responsibilities:
    - Create new token records
    - Update token scam status 
    - Update trading status changes
    - Create and update pool records
    - Handle pool scam status updates
    - Manage token-pool relationships
    """
    
    def __init__(self, logger: Optional[logging.Logger] = None):
        self.logger = logger or logging.getLogger(__name__)
        self.Session = get_db_session_maker(db='eth_db')
    
    def _get_or_create_address_id(self, session, address_str: str, is_contract: bool = True) -> Optional[int]:
        """Get or create address ID for creator address.
        
        Args:
            session: Database session
            address_str: Address string
            is_contract: Whether the address is a contract (default True for token creators)
        """
        if not address_str:
            return None
            
        try:
            # Use INSERT ON CONFLICT for atomic operation to prevent race conditions
            # This handles concurrent inserts of the same address gracefully
            result = session.execute(
                text("""
                INSERT INTO eth_db.addresses (address, is_contract) 
                VALUES (:addr, :is_contract) 
                ON CONFLICT (address) DO UPDATE 
                SET is_contract = COALESCE(addresses.is_contract, :is_contract)
                RETURNING address_id
                """),
                {"addr": address_str, "is_contract": is_contract}
            )
            return result.fetchone()[0]
            
        except Exception as e:
            # Fallback to SELECT if INSERT fails for any other reason
            try:
                result = session.execute(
                    text("SELECT address_id FROM eth_db.addresses WHERE address = :addr"),
                    {"addr": address_str}
                ).fetchone()
                if result:
                    return result[0]
            except:
                pass
            self.logger.error(f"Error handling address {address_str}: {e}")
            return None
    
    def create_or_update_token(self, token_data: Dict[str, Any]) -> bool:
        """
        Create new token or update existing token status.
        
        Args:
            token_data: Dict containing token information
                Required keys: contract_address, is_scam, scam_label
                Optional keys: creator_address, creation_txn, trading_enabled_txn
        
        Returns:
            bool: Success status
        """
        # DEBUG: Log what we're receiving
        # self.logger.debug(f"TokenStatusWriter.create_or_update_token called with: {token_data}")
        
        max_retries = 6
        retry_delays = [0.5, 1.0, 1.5, 2.0, 2.5, 2.5]  # Total: 10 seconds
        
        for attempt in range(max_retries):
            try:
                with self.Session() as session:
                    token_address = token_data["contract_address"]
                    
                    # Get creator address_id if provided
                    # Note: Creator addresses are typically EOAs (not contracts)
                    creator_address_id = None
                    if "creator_address" in token_data and token_data["creator_address"]:
                        creator_address_id = self._get_or_create_address_id(
                            session, token_data["creator_address"], is_contract=False
                        )
                    
                    # Use UPSERT to handle both create and update - covers actual token metadata
                    session.execute(
                    text("""
                    INSERT INTO eth_db.tokens 
                    (contract_address, creator_address_id, is_scam, scam_label, 
                     creation_txn, trading_enabled_txn)
                    VALUES (:contract_address, :creator_address_id, :is_scam, :scam_label,
                            :creation_txn, :trading_enabled_txn)
                    ON CONFLICT (contract_address) DO UPDATE SET
                        is_scam = EXCLUDED.is_scam,
                        scam_label = EXCLUDED.scam_label,
                        creation_txn = COALESCE(EXCLUDED.creation_txn, tokens.creation_txn),
                        trading_enabled_txn = COALESCE(EXCLUDED.trading_enabled_txn, tokens.trading_enabled_txn),
                        creator_address_id = COALESCE(EXCLUDED.creator_address_id, tokens.creator_address_id)
                    """),
                    {
                        "contract_address": token_address,
                        "creator_address_id": creator_address_id,
                        "is_scam": token_data["is_scam"],
                        "scam_label": token_data["scam_label"],
                        "creation_txn": token_data.get("creation_txn"),
                        "trading_enabled_txn": token_data.get("trading_enabled_txn")
                    }
                    )
                    session.commit()
                    return True
                    
            except Exception as e:
                # Check if it's a foreign key violation
                if isinstance(e.__cause__, ForeignKeyViolation) and "creation_txn" in str(e):
                    if attempt < max_retries - 1:
                        # Don't log retry attempts - just retry silently
                        time.sleep(retry_delays[attempt])
                        continue
                    else:
                        self.logger.error(f"Failed to create/update token {token_data.get('contract_address')} "
                                        f"after {max_retries} attempts. Foreign key constraint: {e}")
                else:
                    self.logger.error(f"Error creating/updating token {token_data.get('contract_address')}: {e}")
                return False
        
        # Should not reach here
        return False
    
    def update_scam_status(self, token_address: str, is_scam: bool, scam_label: Optional[str] = None) -> bool:
        """
        Update only the scam status of an existing token.
        
        Args:
            token_address: Token contract address
            is_scam: Whether token is a scam
            scam_label: Reason for scam classification
        
        Returns:
            bool: Success status
        """
        try:
            with self.Session() as session:
                result = session.execute(
                    text("""
                    UPDATE eth_db.tokens 
                    SET is_scam = :is_scam, scam_label = :scam_label
                    WHERE contract_address = :contract_address
                    """),
                    {
                        "contract_address": token_address,
                        "is_scam": is_scam,
                        "scam_label": scam_label
                    }
                )
                session.commit()
                
                if result.rowcount > 0:
                    return True
                else:
                    self.logger.warning(f"No token found to update: {token_address}")
                    return False
                
        except Exception as e:
            self.logger.error(f"Error updating scam status for token {token_address}: {e}")
            return False
    
    def batch_update_scam_status(self, token_updates: Dict[str, Dict[str, Any]]) -> int:
        """
        Batch update scam status for multiple tokens.
        
        Args:
            token_updates: Dict mapping token_address to {"is_scam": bool, "scam_label": str}
        
        Returns:
            int: Number of tokens successfully updated
        """
        if not token_updates:
            return 0
            
        success_count = 0
        try:
            with self.Session() as session:
                for token_address, update_data in token_updates.items():
                    try:
                        result = session.execute(
                            text("""
                            UPDATE eth_db.tokens 
                            SET is_scam = :is_scam, scam_label = :scam_label
                            WHERE contract_address = :contract_address
                            """),
                            {
                                "contract_address": token_address,
                                "is_scam": update_data["is_scam"],
                                "scam_label": update_data["scam_label"]
                            }
                        )
                        
                        if result.rowcount > 0:
                            success_count += 1
                        else:
                            self.logger.warning(f"No token found to update: {token_address}")
                            
                    except Exception as e:
                        self.logger.error(f"Error updating token {token_address}: {e}")
                        continue
                
                session.commit()
                if success_count < len(token_updates):
                    self.logger.warning(f"Batch update only succeeded for {success_count}/{len(token_updates)} tokens")
                
        except Exception as e:
            self.logger.error(f"Error in batch update: {e}")
            
        return success_count
    
    def get_token_status(self, token_address: str) -> Optional[Dict[str, Any]]:
        """
        Get current token status from database.
        
        Args:
            token_address: Token contract address
            
        Returns:
            Dict with token status or None if not found
        """
        try:
            with self.Session() as session:
                result = session.execute(
                    text("""
                    SELECT contract_address, is_scam, scam_label, creation_txn, trading_enabled_txn
                    FROM eth_db.tokens 
                    WHERE contract_address = :contract_address
                    """),
                    {"contract_address": token_address}
                ).fetchone()
                
                if result:
                    return {
                        "contract_address": result[0],
                        "is_scam": result[1],
                        "scam_label": result[2],
                        "creation_txn": result[3],
                        "trading_enabled_txn": result[4]
                    }
                return None
                
        except Exception as e:
            self.logger.error(f"Error getting token status for {token_address}: {e}")
            return None
    
    def create_or_update_token_with_pools(self, token_data: Dict[str, Any], pools_data: Dict[str, Dict[str, Any]]) -> bool:
        """
        Create or update token along with its associated pools in a single transaction.
        
        This is the main method for live token tracking - handles both token and pool persistence.
        
        Args:
            token_data: Dict containing token information
                Required keys: contract_address, is_scam, scam_label
                Optional keys: creator_address, creation_txn, trading_enabled_txn
            pools_data: Dict mapping pool_address to pool information
                Each pool dict should contain:
                Required: pool_type, denom_address, denom_reserve, token_reserve
                Optional: pool_id (for V4), is_scam, scam_label
        
        Returns:
            bool: Success status
        """
        if not pools_data:
            # If no pools, just update token
            return self.create_or_update_token(token_data)
        
        max_retries = 6
        retry_delays = [0.5, 1.0, 1.5, 2.0, 2.5, 2.5]  # Total: 10 seconds
        
        for attempt in range(max_retries):
            try:
                with self.Session() as session:
                    token_address = token_data["contract_address"]
                    
                    # 1. Handle token creation/update (same as before)
                    creator_address_id = None
                    if "creator_address" in token_data and token_data["creator_address"]:
                        creator_address_id = self._get_or_create_address_id(
                            session, token_data["creator_address"]
                        )
                    
                    # Create/update token
                    session.execute(
                        text("""
                        INSERT INTO eth_db.tokens 
                        (contract_address, creator_address_id, is_scam, scam_label, 
                         creation_txn, trading_enabled_txn)
                        VALUES (:contract_address, :creator_address_id, :is_scam, :scam_label,
                                :creation_txn, :trading_enabled_txn)
                        ON CONFLICT (contract_address) DO UPDATE SET
                            is_scam = EXCLUDED.is_scam,
                            scam_label = EXCLUDED.scam_label,
                            creation_txn = COALESCE(EXCLUDED.creation_txn, tokens.creation_txn),
                            trading_enabled_txn = COALESCE(EXCLUDED.trading_enabled_txn, tokens.trading_enabled_txn),
                            creator_address_id = COALESCE(EXCLUDED.creator_address_id, tokens.creator_address_id)
                        """),
                        {
                            "contract_address": token_address,
                            "creator_address_id": creator_address_id,
                            "is_scam": token_data["is_scam"],
                            "scam_label": token_data["scam_label"],
                            "creation_txn": token_data.get("creation_txn"),
                            "trading_enabled_txn": token_data.get("trading_enabled_txn")
                        }
                    )
                    
                    # 2. Handle pools creation/update
                    for pool_address, pool_info in pools_data.items():
                        try:
                            # Determine pair token address from denom_address
                            pair_token_address = pool_info["denom_address"]
                            
                            # For V4 pools, pool_address might be a display address, use pool_id
                            pool_id_value = pool_info.get("pool_id") if pool_info["pool_type"] == "V4" else None
                            actual_pool_address = pool_address if pool_info["pool_type"] in ["V2", "V3"] else None
                            
                            # Handle V2/V3 pools (use pool_address)
                            if pool_info["pool_type"] in ["V2", "V3"] and actual_pool_address:
                                session.execute(
                                    text("""
                                    INSERT INTO eth_db.pools 
                                    (pool_address, pool_id, pool_type, token_address, pair_token_address, fee_tier,
                                     is_scam, scam_label, scam_block, scam_tx_hash,
                                     trading_enabled, trading_enabled_block, trading_enabled_txn)
                                    VALUES (:pool_address, :pool_id, :pool_type, :token_address, :pair_token_address, :fee_tier,
                                            :is_scam, :scam_label, :scam_block, :scam_tx_hash,
                                            :trading_enabled, :trading_enabled_block, :trading_enabled_txn)
                                    ON CONFLICT (pool_address) DO UPDATE SET
                                        pool_type = EXCLUDED.pool_type,
                                        pair_token_address = EXCLUDED.pair_token_address,
                                        fee_tier = COALESCE(EXCLUDED.fee_tier, pools.fee_tier),
                                        is_scam = COALESCE(EXCLUDED.is_scam, pools.is_scam),
                                        scam_label = COALESCE(EXCLUDED.scam_label, pools.scam_label),
                                        scam_block = COALESCE(EXCLUDED.scam_block, pools.scam_block),
                                        scam_tx_hash = COALESCE(EXCLUDED.scam_tx_hash, pools.scam_tx_hash),
                                        trading_enabled = COALESCE(EXCLUDED.trading_enabled, pools.trading_enabled),
                                        trading_enabled_block = COALESCE(EXCLUDED.trading_enabled_block, pools.trading_enabled_block),
                                        trading_enabled_txn = COALESCE(EXCLUDED.trading_enabled_txn, pools.trading_enabled_txn)
                                    """),
                                    {
                                        "pool_address": actual_pool_address,
                                        "pool_id": pool_id_value,
                                        "pool_type": pool_info["pool_type"],
                                        "token_address": token_address,
                                        "pair_token_address": pair_token_address,
                                        "fee_tier": pool_info.get("fee_tier"),
                                        "is_scam": pool_info.get("is_scam", False),
                                        "scam_label": pool_info.get("scam_label"),
                                        "scam_block": pool_info.get("scam_block"),
                                        "scam_tx_hash": pool_info.get("scam_tx_hash"),
                                        "trading_enabled": pool_info.get("trading_enabled", False),
                                        "trading_enabled_block": pool_info.get("trading_enabled_block"),
                                        "trading_enabled_txn": pool_info.get("trading_enabled_txn")
                                    }
                                )
                            
                            # Handle V4 pools (use pool_id)
                            elif pool_info["pool_type"] == "V4" and pool_id_value:
                                session.execute(
                                    text("""
                                    INSERT INTO eth_db.pools 
                                    (pool_address, pool_id, pool_type, token_address, pair_token_address, fee_tier,
                                     is_scam, scam_label, scam_block, scam_tx_hash,
                                     trading_enabled, trading_enabled_block, trading_enabled_txn)
                                    VALUES (:pool_address, :pool_id, :pool_type, :token_address, :pair_token_address, :fee_tier,
                                            :is_scam, :scam_label, :scam_block, :scam_tx_hash,
                                            :trading_enabled, :trading_enabled_block, :trading_enabled_txn)
                                    ON CONFLICT (pool_id) DO UPDATE SET
                                        pool_type = EXCLUDED.pool_type,
                                        pair_token_address = EXCLUDED.pair_token_address,
                                        fee_tier = COALESCE(EXCLUDED.fee_tier, pools.fee_tier),
                                        is_scam = COALESCE(EXCLUDED.is_scam, pools.is_scam),
                                        scam_label = COALESCE(EXCLUDED.scam_label, pools.scam_label),
                                        scam_block = COALESCE(EXCLUDED.scam_block, pools.scam_block),
                                        scam_tx_hash = COALESCE(EXCLUDED.scam_tx_hash, pools.scam_tx_hash),
                                        trading_enabled = COALESCE(EXCLUDED.trading_enabled, pools.trading_enabled),
                                        trading_enabled_block = COALESCE(EXCLUDED.trading_enabled_block, pools.trading_enabled_block),
                                        trading_enabled_txn = COALESCE(EXCLUDED.trading_enabled_txn, pools.trading_enabled_txn)
                                    """),
                                    {
                                        "pool_address": actual_pool_address,
                                        "pool_id": pool_id_value,
                                        "pool_type": pool_info["pool_type"],
                                        "token_address": token_address,
                                        "pair_token_address": pair_token_address,
                                        "fee_tier": pool_info.get("fee_tier"),
                                        "is_scam": pool_info.get("is_scam", False),
                                        "scam_label": pool_info.get("scam_label"),
                                        "scam_block": pool_info.get("scam_block"),
                                        "scam_tx_hash": pool_info.get("scam_tx_hash"),
                                        "trading_enabled": pool_info.get("trading_enabled", False),
                                        "trading_enabled_block": pool_info.get("trading_enabled_block"),
                                        "trading_enabled_txn": pool_info.get("trading_enabled_txn")
                                    }
                                )
                                
                        except Exception as e:
                            self.logger.error(f"Error updating pool {pool_address} for token {token_address}: {e}")
                            # Continue with other pools rather than failing entire transaction
                            continue
                    
                    session.commit()
                    return True
                    
            except Exception as e:
                # Check if it's a foreign key violation
                if isinstance(e.__cause__, ForeignKeyViolation) and "creation_txn" in str(e):
                    if attempt < max_retries - 1:
                        # Don't log retry attempts - just retry silently
                        time.sleep(retry_delays[attempt])
                        continue
                    else:
                        self.logger.error(f"Failed to create/update token {token_data.get('contract_address')} "
                                        f"after {max_retries} attempts. Foreign key constraint: {e}")
                else:
                    self.logger.error(f"Error creating/updating token with pools {token_data.get('contract_address')}: {e}")
                return False
        
        # Should not reach here
        return False
    
    def batch_update_pool_scam_status(self, pool_updates: Dict[str, Dict[str, Any]]) -> int:
        """
        Batch update scam status for multiple pools.
        
        Note: This would require adding scam status fields to the pools table schema.
        Currently pools table doesn't have is_scam/scam_label fields.
        
        Args:
            pool_updates: Dict mapping pool_address to {"is_scam": bool, "scam_label": str}
        
        Returns:
            int: Number of pools successfully updated
        """
        # TODO: Implement when pools table gets scam status fields
        self.logger.warning("Pool scam status updates not implemented - pools table needs is_scam/scam_label fields")
        return 0
    
