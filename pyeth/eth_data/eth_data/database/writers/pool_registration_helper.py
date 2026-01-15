"""
Pool Registration Helper

Ensures pools are registered in eth_db.pools before writing positions.
Handles pool type detection and caches pool IDs for performance.
"""

from typing import Optional, Dict, Tuple
from web3 import Web3
import psycopg2
from psycopg2.extras import RealDictCursor


class PoolRegistrationHelper:
    """Helper class to manage pool registration in eth_db."""
    
    def __init__(self, eth_db_conn, logger):
        """
        Initialize with connection to eth_db.
        
        Args:
            eth_db_conn: psycopg2 connection to eth_db
            logger: Logger instance
        """
        self.conn = eth_db_conn
        self.cur = self.conn.cursor(cursor_factory=RealDictCursor)
        self.logger = logger
        
        # Cache to avoid repeated lookups
        self.pool_cache: Dict[str, int] = {}  # pool_address -> pool_id
        self.v4_pool_cache: Dict[str, int] = {}  # pool_id_hex -> pool_id
        
    def ensure_pool_registered(self, token_address: str, pool_address: Optional[str], 
                             pool_type: Optional[str] = None,
                             pool_id_hex: Optional[str] = None,
                             pair_token_address: Optional[str] = None,
                             fee_tier: Optional[int] = None) -> Optional[int]:
        """
        Ensures pool is registered in eth_db.pools and returns pool_id.
        
        Args:
            token_address: Token contract address
            pool_address: Pool address (None for V4 pools)
            pool_type: Pool type (V2, V3, V4) - will try to detect if not provided
            pool_id_hex: V4 pool ID as hex string
            pair_token_address: Pair token address (usually WETH)
            fee_tier: Fee tier for V3/V4 pools
            
        Returns:
            pool_id if successful, None otherwise
        """
        try:
            # Convert to checksum addresses
            token_address = Web3.to_checksum_address(token_address)
            if pool_address:
                pool_address = Web3.to_checksum_address(pool_address)
            
            # Check cache first
            if pool_address and pool_address in self.pool_cache:
                return self.pool_cache[pool_address]
            elif pool_id_hex and pool_id_hex in self.v4_pool_cache:
                return self.v4_pool_cache[pool_id_hex]
            
            # Check if pool exists in database
            pool_id = self._get_existing_pool(pool_address, pool_id_hex)
            if pool_id:
                self._update_cache(pool_address, pool_id_hex, pool_id)
                return pool_id
            
            # Detect pool type if not provided
            if not pool_type:
                pool_type = self._detect_pool_type(pool_address, pool_id_hex, fee_tier)
            
            # Set default pair token if not provided
            if not pair_token_address:
                pair_token_address = '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'  # WETH
            else:
                pair_token_address = Web3.to_checksum_address(pair_token_address)
            
            # Register new pool
            pool_id = self._register_pool(
                token_address, pool_address, pool_type, 
                pool_id_hex, pair_token_address, fee_tier
            )
            
            if pool_id:
                self._update_cache(pool_address, pool_id_hex, pool_id)
                self.logger.info(
                    f"Registered new {pool_type} pool: "
                    f"token={token_address[:10]}... "
                    f"pool={pool_address[:10] if pool_address else pool_id_hex[:10]}..."
                )
            
            return pool_id
            
        except Exception as e:
            self.logger.error(f"Failed to ensure pool registered: {e}")
            return None
    
    def _get_existing_pool(self, pool_address: Optional[str], 
                          pool_id_hex: Optional[str]) -> Optional[int]:
        """Check if pool already exists in database."""
        try:
            if pool_address:
                self.cur.execute(
                    "SELECT id FROM eth_db.pools WHERE pool_address = %s",
                    (pool_address,)
                )
            elif pool_id_hex:
                self.cur.execute(
                    "SELECT id FROM eth_db.pools WHERE pool_id = %s",
                    (pool_id_hex,)
                )
            else:
                return None
            
            result = self.cur.fetchone()
            return result['id'] if result else None
            
        except Exception as e:
            self.logger.error(f"Error checking existing pool: {e}")
            return None
    
    def _detect_pool_type(self, pool_address: Optional[str], 
                         pool_id_hex: Optional[str],
                         fee_tier: Optional[int]) -> str:
        """Detect pool type based on available information."""
        if pool_id_hex and not pool_address:
            return 'V4'
        elif fee_tier is not None:
            return 'V3'
        else:
            return 'V2'
    
    def _register_pool(self, token_address: str, pool_address: Optional[str],
                      pool_type: str, pool_id_hex: Optional[str],
                      pair_token_address: str, fee_tier: Optional[int]) -> Optional[int]:
        """Register a new pool in the database."""
        try:
            if pool_type in ['V2', 'V3']:
                self.cur.execute("""
                    INSERT INTO eth_db.pools 
                    (pool_address, pool_type, token_address, pair_token_address, fee_tier)
                    VALUES (%s, %s, %s, %s, %s)
                    RETURNING id
                """, (pool_address, pool_type, token_address, pair_token_address, fee_tier))
            else:  # V4
                self.cur.execute("""
                    INSERT INTO eth_db.pools 
                    (pool_id, pool_type, token_address, pair_token_address, fee_tier)
                    VALUES (%s, %s, %s, %s, %s)
                    RETURNING id
                """, (pool_id_hex, pool_type, token_address, pair_token_address, fee_tier))
            
            self.conn.commit()
            result = self.cur.fetchone()
            return result['id'] if result else None
            
        except psycopg2.IntegrityError as e:
            # Pool might have been created by another process
            self.conn.rollback()
            return self._get_existing_pool(pool_address, pool_id_hex)
        except Exception as e:
            self.conn.rollback()
            self.logger.error(f"Failed to register pool: {e}")
            return None
    
    def _update_cache(self, pool_address: Optional[str], 
                     pool_id_hex: Optional[str], pool_id: int):
        """Update the cache with pool information."""
        if pool_address:
            self.pool_cache[pool_address] = pool_id
        if pool_id_hex:
            self.v4_pool_cache[pool_id_hex] = pool_id
    
    def get_pool_info(self, pool_id: int) -> Optional[Dict]:
        """Get pool information by ID."""
        try:
            self.cur.execute("""
                SELECT pool_address, pool_id, pool_type, token_address, 
                       pair_token_address, fee_tier
                FROM eth_db.pools
                WHERE id = %s
            """, (pool_id,))
            
            result = self.cur.fetchone()
            return dict(result) if result else None
            
        except Exception as e:
            self.logger.error(f"Failed to get pool info: {e}")
            return None