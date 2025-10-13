"""
Token Tracking Cache - Maintains live token and pool state

This cache tracks all active tokens and their pools, with intelligent
eviction of scam tokens after they become inactive.

Key Features:
1. Token-centric organization (one token -> multiple pools)
2. Tracks all tokens (healthy and scam)
3. Evicts scam tokens only after 5000 blocks of inactivity
"""

import asyncio
from typing import Dict, Optional, Any, List, Set
from collections import OrderedDict
import time
from dataclasses import dataclass, field


@dataclass
class TaxChange:
    """Record of a tax change event"""
    block_number: int
    timestamp: int
    transaction_hash: str
    changer_address: str
    old_buy_tax: float
    new_buy_tax: float
    old_sell_tax: float
    new_sell_tax: float
    
    def __post_init__(self):
        """Validate tax percentages are in valid range"""
        for tax_field, value in [
            ('old_buy_tax', self.old_buy_tax),
            ('new_buy_tax', self.new_buy_tax),
            ('old_sell_tax', self.old_sell_tax),
            ('new_sell_tax', self.new_sell_tax)
        ]:
            if not 0 <= value <= 100:
                raise ValueError(f"{tax_field} must be between 0 and 100, got {value}")
    
@dataclass 
class PendingTaxChange:
    """Tax change detected in mempool but not yet executed"""
    transaction_hash: str
    from_address: str
    predicted_buy_tax: float
    predicted_sell_tax: float
    confidence: float
    detection_timestamp: int
    function_selector: str  # The tax-setting function being called
    
    def __post_init__(self):
        """Validate tax percentages and confidence are in valid range"""
        if not 0 <= self.predicted_buy_tax <= 100:
            raise ValueError(f"predicted_buy_tax must be between 0 and 100, got {self.predicted_buy_tax}")
        if not 0 <= self.predicted_sell_tax <= 100:
            raise ValueError(f"predicted_sell_tax must be between 0 and 100, got {self.predicted_sell_tax}")
        if not 0 <= self.confidence <= 1:
            raise ValueError(f"confidence must be between 0 and 1, got {self.confidence}")
    

@dataclass
class PoolSpecificInfo:
    """Pool-specific information (varies per pool)"""
    pool_address: str
    pool_type: str  # V2, V3, V4
    denom_currency: str  # WETH, USDC, USDT, etc.
    denom_address: str
    denom_reserve: float
    token_reserve: float
    latest_block_number: int
    last_update_time: float
    pool_id: Optional[str] = None  # For V4 pools
    is_scam: bool = False
    scam_label: Optional[str] = None
    # Per-pool trading enabled tracking
    trading_enabled: bool = False
    trading_enabled_block: Optional[int] = None
    trading_enabled_tx: Optional[str] = None
    # LP token approval tracking (V2 pools only)
    lp_tokens_approved_percentage: Optional[float] = None


@dataclass
class TokenInfo:
    """Token information (shared across all pools)"""
    token_address: str
    token_symbol: str
    token_name: str
    token_decimals: int
    total_supply: int
    
    # Creation info
    creator_address: Optional[str]
    creation_block: Optional[int]
    creation_timestamp: Optional[int]
    creation_tx: Optional[str]
    
    # Trading info
    trading_enabled: bool
    trading_enabled_block: Optional[int]
    trading_enabled_tx: Optional[str]
    
    # Ownership info
    current_owner: Optional[str]
    ownership_renounced: bool
    renouncement_block: Optional[int]
    
    # Scam tracking
    is_scam: bool = False
    scam_label: Optional[str] = None
    
    # Tax tracking
    current_buy_tax: Optional[float] = None
    current_sell_tax: Optional[float] = None
    tax_history: List[TaxChange] = field(default_factory=list)  # Last 10 changes
    pending_tax_changes: List[PendingTaxChange] = field(default_factory=list)  # From mempool
    last_tax_change_block: Optional[int] = None
    tax_setter_addresses: Set[str] = field(default_factory=set)  # Addresses that have set taxes
    
    # Pool tracking
    pools: Dict[str, PoolSpecificInfo] = field(default_factory=dict)
    
    # Activity tracking
    latest_activity_block: int = 0


class TokenTrackingCache:
    """
    Cache for token and pool information with intelligent scam eviction.
    
    Token-centric organization:
    - Each token has one entry with all its metadata
    - Each token tracks multiple pools
    - Scam tokens kept for 5000 blocks after last activity
    """
    
    INACTIVITY_THRESHOLD = 5000  # Blocks before removing inactive scam tokens
    
    def __init__(self, logger):
        self.logger = logger
        
        # Main cache: token_address -> TokenInfo
        self._token_cache: OrderedDict[str, TokenInfo] = OrderedDict()
        
        # Pool to token mapping for quick lookups
        self._pool_to_token: Dict[str, str] = {}
        
        # Track tokens updated in current block for incremental publishing
        self._updated_tokens_current_block: Dict[str, TokenInfo] = {}
        
        self._lock = asyncio.Lock()
    
    async def update_from_token_objects(self, updated_tokens: Dict[str, Any], block_number: int) -> None:
        """
        Process ERC20Token objects from LiveTokenTracker and update cache.
        
        Args:
            updated_tokens: Dict mapping token addresses to ERC20Token objects
            block_number: The block number these updates came from
        """
        if not updated_tokens:
            return
        
        async with self._lock:
            # Clear previous block's updated tokens and start tracking new ones
            self._updated_tokens_current_block.clear()
            for token_address, token in updated_tokens.items():
                # Get pool addresses
                pool_addresses = token.token_data.pool_addresses
                if not pool_addresses:
                    continue
                
                # Get or create token info
                if token_address not in self._token_cache:
                    # Create new token entry with metadata
                    token_info = TokenInfo(
                        token_address=token_address,
                        token_symbol=token.token_data.symbol,
                        token_name=token.token_data.name,
                        token_decimals=token.token_data.decimals,
                        total_supply=token.token_data.total_supply,
                        
                        # Creation info
                        creator_address=token.token_data.creator_address,
                        creation_block=token.token_data.creation_block,
                        creation_timestamp=token.token_data.creation_timestamp,
                        creation_tx=token.token_data.creation_tx,
                        
                        # Trading info - DEPRECATED (now per-pool)
                        # trading_enabled tracked per pool in pool_info
                        trading_enabled=False,  # Deprecated - check pool level
                        trading_enabled_block=None,  # Deprecated
                        trading_enabled_tx=None,  # Deprecated
                        
                        # Ownership info
                        current_owner=token.token_data.current_owner,
                        ownership_renounced=token.token_data.ownership_renounced,
                        renouncement_block=token.token_data.renouncement_block,
                        
                        # Scam info
                        is_scam=token.token_data.is_scam,
                        scam_label=token.token_data.scam_label,
                        
                        # Tax info (initialize from token data if available)
                        current_buy_tax=getattr(token.token_data, 'buy_tax', None),
                        current_sell_tax=getattr(token.token_data, 'sell_tax', None),
                        
                        latest_activity_block=block_number
                    )
                    self._token_cache[token_address] = token_info
                else:
                    # Update existing token
                    token_info = self._token_cache[token_address]
                    
                    # Update fields that might change
                    # token_info.trading_enabled = DEPRECATED - now per-pool
                    token_info.current_owner = token.token_data.current_owner
                    token_info.ownership_renounced = token.token_data.ownership_renounced
                    token_info.is_scam = token.token_data.is_scam
                    token_info.scam_label = token.token_data.scam_label
                    token_info.latest_activity_block = block_number
                    
                    # Update tax info if available
                    new_buy_tax = getattr(token.token_data, 'buy_tax', None)
                    new_sell_tax = getattr(token.token_data, 'sell_tax', None)
                    
                    # Check if taxes have changed
                    if new_buy_tax is not None and new_sell_tax is not None:
                        if (token_info.current_buy_tax != new_buy_tax or 
                            token_info.current_sell_tax != new_sell_tax):
                            # Record tax change
                            if token_info.current_buy_tax is not None and token_info.current_sell_tax is not None:
                                tax_change = TaxChange(
                                    block_number=block_number,
                                    timestamp=int(time.time()),
                                    transaction_hash=getattr(token.token_data, 'last_tx_hash', ''),
                                    changer_address=token_info.current_owner or '',
                                    old_buy_tax=token_info.current_buy_tax,
                                    new_buy_tax=new_buy_tax,
                                    old_sell_tax=token_info.current_sell_tax,
                                    new_sell_tax=new_sell_tax
                                )
                                token_info.tax_history.append(tax_change)
                                # Keep only last 10 tax changes
                                token_info.tax_history = token_info.tax_history[-10:]
                                token_info.last_tax_change_block = block_number
                                
                                # Add tax setter address
                                if token_info.current_owner:
                                    token_info.tax_setter_addresses.add(token_info.current_owner)
                                
                            
                            # Update current taxes
                            token_info.current_buy_tax = new_buy_tax
                            token_info.current_sell_tax = new_sell_tax
                
                # Process pools
                pool_info_dict = token.token_data.get_pool_info_dict()
                
                for pool_address in pool_addresses:
                    try:
                        # Get pool info
                        pool_info = pool_info_dict[pool_address]
                        denom_reserve = token.token_data.get_pool_reserve(pool_address)
                        
                        if denom_reserve is None:
                            continue
                        
                        # Get scam info and trading enabled from the pool object
                        pool_is_scam = False
                        pool_scam_label = None
                        pool_trading_enabled = False
                        pool_trading_enabled_block = None
                        pool_trading_enabled_tx = None
                        lp_tokens_approved_percentage = None
                        
                        if token.token_data.pool_manager:
                            pool_obj = token.token_data.pool_manager.get_pool(pool_address)
                            if pool_obj:
                                pool_is_scam = pool_obj.is_scam
                                if pool_is_scam:
                                    pool_scam_label = pool_obj.scam_label or "Low liquidity"
                                # Get trading enabled info
                                pool_trading_enabled = pool_obj.trading_enabled
                                pool_trading_enabled_block = pool_obj.trading_enabled_block if pool_obj.trading_enabled else None
                                pool_trading_enabled_tx = pool_obj.trading_enabled_tx if pool_obj.trading_enabled else None
                                
                                # Get LP approval percentage for V2 pools
                                if pool_obj.get_protocol() == 'Uniswap-V2':
                                    lp_tokens_approved_percentage = pool_obj.get_lp_approved_percentage()
                        
                        # Create/update pool info
                        pool_data = PoolSpecificInfo(
                            pool_address=pool_address,
                            pool_type=pool_info['pool_type'],
                            denom_currency=pool_info['denom_currency'],
                            denom_address=pool_info['denom_address'],
                            denom_reserve=float(denom_reserve),
                            token_reserve=float(token.token_data.get_pool_token_reserve(pool_address)),
                            latest_block_number=block_number,
                            last_update_time=time.time(),
                            pool_id=pool_info['pool_id'] if pool_info['pool_type'] == 'V4' else None,
                            is_scam=pool_is_scam,
                            scam_label=pool_scam_label,
                            trading_enabled=pool_trading_enabled,
                            trading_enabled_block=pool_trading_enabled_block,
                            trading_enabled_tx=pool_trading_enabled_tx,
                            lp_tokens_approved_percentage=lp_tokens_approved_percentage
                        )
                        
                        # Update token's pools
                        token_info.pools[pool_address] = pool_data
                        
                        # Update pool->token mapping
                        self._pool_to_token[pool_address] = token_address
                        
                    except Exception as e:
                        self.logger.error(f"Error processing pool {pool_address} for token {token_address}: {e}")
                
                # Move token to end for LRU
                self._token_cache.move_to_end(token_address)
                
                # Track this token as updated in current block
                self._updated_tokens_current_block[token_address] = token_info
            
            # Periodic cleanup
            if block_number % 100 == 0:
                await self._cleanup_inactive_scams(block_number)
    
    async def _cleanup_inactive_scams(self, current_block: int) -> None:
        """Remove scam tokens that have been inactive for INACTIVITY_THRESHOLD blocks."""
        tokens_to_remove = []
        
        for token_address, token_info in self._token_cache.items():
            if token_info.is_scam:
                blocks_inactive = current_block - token_info.latest_activity_block
                if blocks_inactive >= self.INACTIVITY_THRESHOLD:
                    tokens_to_remove.append(token_address)
        
        # Remove inactive scam tokens
        for token_address in tokens_to_remove:
            token_info = self._token_cache[token_address]
            
            # Remove pool mappings
            for pool_address in token_info.pools:
                self._pool_to_token.pop(pool_address, None)
            
            # Remove token
            del self._token_cache[token_address]
            
            self.logger.info(f"Scam token {token_info.token_address} is removed from cache")
    
    # Methods needed by publisher
    
    def get_token(self, token_address: str) -> Optional[TokenInfo]:
        """Get specific token info (for REQ/REP queries)."""
        return self._token_cache.get(token_address)
    
    def get_pool(self, pool_address: str) -> Optional[PoolSpecificInfo]:
        """Get specific pool info (for REQ/REP queries)."""
        token_address = self._pool_to_token.get(pool_address)
        if token_address and token_address in self._token_cache:
            return self._token_cache[token_address].pools.get(pool_address)
        return None
    
    def get_all_tokens(self) -> Dict[str, TokenInfo]:
        """Get all tokens (for mempool data requests)."""
        return dict(self._token_cache)
    
    def _format_token_for_publishing(self, token_addr: str, token_info: TokenInfo) -> Dict[str, Any]:
        """Helper method to format a single token for publishing."""
        token_data = {
            'token_address': token_addr,
            'token_symbol': token_info.token_symbol,
            'token_name': token_info.token_name,
            'token_decimals': token_info.token_decimals,
            'total_supply': token_info.total_supply,
            
            'creator_address': token_info.creator_address,
            'creation_block': token_info.creation_block,
            'creation_timestamp': token_info.creation_timestamp,
            'creation_tx': token_info.creation_tx,
            
            # NOTE: Trading enabled is now tracked per-pool, not per-token
            # See pools[pool_address]['trading_enabled'] for pool-specific status
            # 'trading_enabled': token_info.trading_enabled,  # DEPRECATED
            # 'trading_enabled_block': token_info.trading_enabled_block,  # DEPRECATED
            # 'trading_enabled_tx': token_info.trading_enabled_tx,  # DEPRECATED
            
            'current_owner': token_info.current_owner,
            'ownership_renounced': token_info.ownership_renounced,
            'renouncement_block': token_info.renouncement_block,
            
            'is_scam': token_info.is_scam,
            'scam_label': token_info.scam_label,
            'latest_activity_block': token_info.latest_activity_block,
            
            # Tax information
            'current_buy_tax': token_info.current_buy_tax,
            'current_sell_tax': token_info.current_sell_tax,
            'last_tax_change_block': token_info.last_tax_change_block,
            'tax_setter_addresses': list(token_info.tax_setter_addresses),
            
            # Tax history (format for JSON serialization)
            'tax_history': [
                {
                    'block_number': tc.block_number,
                    'timestamp': tc.timestamp,
                    'transaction_hash': tc.transaction_hash,
                    'changer_address': tc.changer_address,
                    'old_buy_tax': tc.old_buy_tax,
                    'new_buy_tax': tc.new_buy_tax,
                    'old_sell_tax': tc.old_sell_tax,
                    'new_sell_tax': tc.new_sell_tax
                }
                for tc in token_info.tax_history
            ],
            
            # Pending tax changes from mempool
            'pending_tax_changes': [
                {
                    'transaction_hash': ptc.transaction_hash,
                    'from_address': ptc.from_address,
                    'predicted_buy_tax': ptc.predicted_buy_tax,
                    'predicted_sell_tax': ptc.predicted_sell_tax,
                    'confidence': ptc.confidence,
                    'detection_timestamp': ptc.detection_timestamp,
                    'function_selector': ptc.function_selector
                }
                for ptc in token_info.pending_tax_changes
            ],
            
            'pools': {}
        }
        
        # Add pools
        for pool_addr, pool_info in token_info.pools.items():
            token_data['pools'][pool_addr] = {
                'pool_address': pool_addr,
                'pool_type': pool_info.pool_type,
                'denom_currency': pool_info.denom_currency,
                'denom_address': pool_info.denom_address,
                'denom_reserve': pool_info.denom_reserve,
                'token_reserve': pool_info.token_reserve,
                'latest_block_number': pool_info.latest_block_number,
                'last_update_time': pool_info.last_update_time,
                'pool_id': pool_info.pool_id,
                'is_scam': pool_info.is_scam,
                'scam_label': pool_info.scam_label,
                # Per-pool trading enabled fields
                'trading_enabled': pool_info.trading_enabled,
                'trading_enabled_block': pool_info.trading_enabled_block,
                'trading_enabled_tx': pool_info.trading_enabled_tx,
                # LP token approval tracking (V2 pools only)
                'lp_tokens_approved_percentage': pool_info.lp_tokens_approved_percentage
            }
        
        return token_data

    async def get_all_tokens_for_publishing(self) -> Dict[str, Dict[str, Any]]:
        """Get all tokens formatted for publishing (for startup/full sync)."""
        async with self._lock:
            result = {}
            for token_addr, token_info in self._token_cache.items():
                result[token_addr] = self._format_token_for_publishing(token_addr, token_info)
            return result

    async def get_updated_tokens_for_publishing(self) -> Dict[str, Dict[str, Any]]:
        """Get only tokens updated in current block for incremental publishing."""
        async with self._lock:
            result = {}
            for token_addr, token_info in self._updated_tokens_current_block.items():
                result[token_addr] = self._format_token_for_publishing(token_addr, token_info)
            return result
    
    async def update_pending_tax_changes(self, token_address: str, pending_changes: List[PendingTaxChange]) -> None:
        """Update pending tax changes for a token from mempool detection."""
        async with self._lock:
            if token_address in self._token_cache:
                token_info = self._token_cache[token_address]
                token_info.pending_tax_changes = pending_changes
                
                
    
    def get_tokens_with_pending_tax_changes(self) -> Dict[str, TokenInfo]:
        """Get all tokens with pending tax changes in mempool."""
        return {
            addr: token for addr, token in self._token_cache.items()
            if token.pending_tax_changes
        }
    
    def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics."""
        total_tokens = len(self._token_cache)
        scam_tokens = sum(1 for t in self._token_cache.values() if t.is_scam)
        total_pools = sum(len(t.pools) for t in self._token_cache.values())
        tokens_with_pending_tax_changes = sum(1 for t in self._token_cache.values() if t.pending_tax_changes)
        
        return {
            'total_tokens': total_tokens,
            'healthy_tokens': total_tokens - scam_tokens,
            'scam_tokens': scam_tokens,
            'total_pools': total_pools,
            'tokens_with_pending_tax_changes': tokens_with_pending_tax_changes,
            'inactivity_threshold_blocks': self.INACTIVITY_THRESHOLD
        }
