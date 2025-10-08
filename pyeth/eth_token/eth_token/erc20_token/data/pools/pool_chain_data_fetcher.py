"""
Pool Chain Data Fetcher

Unified, RPC-free pool information and liquidity fetcher backed by PyReth
and the reth_chain_query provider.

Goals:
- Provide a single place to fetch pool basics (token0/token1/fee) and
  liquidity snapshots for V2/V3/V4.
- Prefer direct MDBX (reth) reads via PyReth ChainQuery; avoid duplicating
  per-pool logic scattered across pool classes.
- Keep a small Web3 fallback path (legacy) where helpful.
"""

import math
from typing import Optional, Dict, Tuple, Any
from eth_utils import to_checksum_address
from eth_data.utils.pyreth_client import PyrethClient
from eth_utils import keccak
from eth_data.chain_utils.common_addresses import canonicalize_dex_pool_type


UNISWAP_V2_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V2')
UNISWAP_V3_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V3')
        

def _format_scaled(raw_value: int, decimals: int) -> str:
    if decimals <= 0:
        return str(raw_value)
    digits = str(raw_value)
    if len(digits) <= decimals:
        digits = digits.zfill(decimals + 1)
        integer_part = '0'
        fractional = digits[-decimals:]
    else:
        integer_part = digits[:-decimals]
        fractional = digits[-decimals:]
    fractional = fractional.rstrip('0')
    if not fractional:
        return integer_part
    return f"{integer_part}.{fractional}"


def _estimate_price_from_tick(tick: int, token0_decimals: int, token1_decimals: int) -> float:
    try:
        price = math.pow(1.0001, int(tick))
        scale = 10 ** (int(token1_decimals) - int(token0_decimals))
        return price / scale
    except Exception:
        return 0.0


class PoolChainDataFetcher:
    """
    Fetches pool configuration data from blockchain contracts.
    
    Used when pools are discovered through swap events rather than creation events,
    requiring us to fetch their configuration from the blockchain.
    """
    
    # Minimal ABIs for pool discovery
    def __init__(self):
        """
        Initialize the discovery service.
        
        Args:
            w3: Web3 instance for blockchain queries
            logger: Logger instance
        """
        # Track discovered pools to avoid repeated queries
        self.pools: Dict[str, Dict] = {}

        # Initialize PyReth ChainQuery once per process for fast reuse
        client = PyrethClient.instance()
        self._chain_query = client.chain_query()

    @property
    def chain_query(self):
        """Expose the underlying PyReth chain query handle."""
        return self._chain_query

    # -----------------------------
    # Normalization helpers
    # -----------------------------
    def _to_checksum(self, addr: Optional[str]) -> Optional[str]:
        return to_checksum_address(addr)

    def _normalize_liquidity_info(self, raw: Any) -> Dict[str, Any]:
        """Normalize PyReth liquidity info into a consistent dict schema.

        Schema:
          - protocol: 'UniswapV2' | 'SushiswapV2' | 'UniswapV3' | 'UniswapV4'
          - pool_address: checksum address (pool for V2/V3, PoolManager for V4)
          - token0: checksum address or None
          - token1: checksum address or None
          - reserve0: str or None (V2/V2-like)
          - reserve1: str or None (V2/V2-like)
          - liquidity: str or None (V3/V4)
          - tick: int or None (V3/V4)
          - block_number: int
          - pool_id: str hex (V4 only, not provided by this call)
        """
        # raw is a PyPoolLiquidityInfo
        protocol = canonicalize_dex_pool_type(raw.protocol)
        pool = raw.pool
        token0 = raw.token0
        token1 = raw.token1
        token0_decimals = raw.token0_decimals
        token1_decimals = raw.token1_decimals
        reserve0_raw = raw.reserve0_raw
        reserve1_raw = raw.reserve1_raw
        reserve0_adjusted = raw.reserve0_scaled
        reserve1_adjusted = raw.reserve1_scaled
        v3_liquidity = raw.v3_liquidity
        tick = raw.tick
        price_1e18 = raw.price_1e18
        block_number = raw.block_number

        data: Dict[str, Any] = {
            'protocol': protocol,
            'pool': self._to_checksum(pool) if isinstance(pool, str) else pool,
            'token0': self._to_checksum(token0) if isinstance(token0, str) else token0,
            'token1': self._to_checksum(token1) if isinstance(token1, str) else token1,
            'token0_decimals': token0_decimals,
            'token1_decimals': token1_decimals,
            'reserve0_raw': reserve0_raw,
            'reserve1_raw': reserve1_raw,
            'reserve0_scaled': reserve0_adjusted,
            'reserve1_scaled': reserve1_adjusted,
            'liquidity': v3_liquidity,
            'tick': tick,
            'price_1e18': price_1e18,
            'block_number': block_number,
        }

        if (
            (reserve0_raw is None or reserve1_raw is None)
            and v3_liquidity is not None
            and tick is not None
            and token0_decimals is not None
            and token1_decimals is not None
            and raw.sqrt_price_x96 is not None
        ):
            try:
                liquidity_int = int(v3_liquidity)
                sqrt_price_x96 = int(raw.sqrt_price_x96)
                if sqrt_price_x96 > 0:
                    reserve0_int = (liquidity_int << 96) // sqrt_price_x96
                    reserve1_int = (liquidity_int * sqrt_price_x96) >> 96
                    reserve0_raw = str(reserve0_int)
                    reserve1_raw = str(reserve1_int)
                    reserve0_adjusted = _format_scaled(reserve0_int, token0_decimals)
                    reserve1_adjusted = _format_scaled(reserve1_int, token1_decimals)
                    data['reserve0_raw'] = reserve0_raw
                    data['reserve1_raw'] = reserve1_raw
                    data['reserve0_scaled'] = reserve0_adjusted
                    data['reserve1_scaled'] = reserve1_adjusted
                    data['price_token1_per_token0'] = _estimate_price_from_tick(
                        tick, token0_decimals, token1_decimals
                    )
            except Exception:
                pass

        return data

    # -----------------------------
    # Unified PyReth-backed helpers
    # -----------------------------
    def get_v2_liquidity(self, pool_address: str, block: Optional[int] = None) -> Dict[str, Any]:
        """Get UniswapV2/SushiswapV2 style liquidity using PyReth (reserves at block).

        Returns a dict with keys: protocol, pool, token0, token1, reserve0, reserve1, block_number.
        """
        info = self._chain_query.get_uniswap_v2_liquidity(pool_address, block)
        if info is None:
            raise RuntimeError(f"Missing V2 liquidity for pool {pool_address}")
        return self._normalize_liquidity_info(info)

    def get_v3_liquidity(self, pool_address: str, fee_tier: int, block: Optional[int] = None) -> Dict[str, Any]:
        """Get UniswapV3 liquidity using PyReth (liquidity, tick at block)."""
        info = self._chain_query.get_uniswap_v3_liquidity(pool_address, int(fee_tier), block)
        if info is None:
            raise RuntimeError(f"Missing V3 liquidity for pool {pool_address}")
        return self._normalize_liquidity_info(info)

    def get_v4_liquidity(self, pool_manager: str, pool_id_hex: str, block: Optional[int] = None) -> Dict[str, Any]:
        """Get UniswapV4 liquidity using PyReth via PoolManager + PoolId."""
        info = self._chain_query.get_uniswap_v4_liquidity(pool_manager, pool_id_hex, block)
        if info is None:
            raise RuntimeError(
                f"Missing V4 liquidity for pool manager {pool_manager} pool {pool_id_hex}"
            )
        out = self._normalize_liquidity_info(info)
        out['pool_id'] = pool_id_hex
        return out
    
    def discover_v2_pool(self, pool_address: str) -> Optional[Dict]:
        """
        Discover V2 pool configuration from blockchain.
        
        Args:
            pool_address: The pool contract address
            
        Returns:
            Dict with token0, token1 addresses or None if discovery fails
        """
        # PyReth route (no RPC), yields token0/1 for V2
        info = self.get_v2_liquidity(pool_address)
        if info is not None and info.get('token0') and info.get('token1'):
            result = {
                'pool_address': self._to_checksum(pool_address),
                'token0': self._to_checksum(info['token0']) if isinstance(info['token0'], str) else info['token0'],
                'token1': self._to_checksum(info['token1']) if isinstance(info['token1'], str) else info['token1'],
                'protocol': UNISWAP_V2_PROTOCOL
            }
            self.pools[f"v2_{pool_address}"] = result
            return result
        return None
    
    def discover_v3_pool(self, pool_address: str) -> Optional[Dict]:
        """
        Discover V3 pool configuration from blockchain.
        
        Args:
            pool_address: The pool contract address
            
        Returns:
            Dict with token0, token1, and fee or None if discovery fails
        """
        # PyReth route for V3
        info = self.get_v3_liquidity(pool_address, fee_tier=3000)
        if info is not None and info.get('token0') and info.get('token1'):
            result = {
                'pool_address': self._to_checksum(pool_address),
                'token0': self._to_checksum(info['token0']) if isinstance(info['token0'], str) else info['token0'],
                'token1': self._to_checksum(info['token1']) if isinstance(info['token1'], str) else info['token1'],
                'fee': 3000,  # If fee not known, default commonly used; caller can override
                'protocol': UNISWAP_V3_PROTOCOL
            }
            self.pools[f"v3_{pool_address}"] = result
            return result
        return None
    
    def discover_pool_for_token(self, pool_address: str, token_address: str, 
                               protocol_hint: Optional[str] = None) -> Optional[Dict]:
        """
        Discover pool configuration and check if it involves the specified token.
        
        Args:
            pool_address: The pool contract address
            token_address: The token we're tracking
            protocol_hint: Optional hint about pool protocol (V2 or V3)
            
        Returns:
            Dict with pool configuration if token is involved, None otherwise
        """
        # NOTE: This method previously relied on Web3 and is not required for unified fetcher.
        # Consider removing or re-implementing with ChainQuery if needed.
        token_address = to_checksum_address(token_address)
        
        # Try protocol hint first if provided
        hint = canonicalize_dex_pool_type(protocol_hint) if protocol_hint else None

        if hint == UNISWAP_V2_PROTOCOL:
            pool_info = self.discover_v2_pool(pool_address)
        elif hint == UNISWAP_V3_PROTOCOL:
            pool_info = self.discover_v3_pool(pool_address)
        else:
            # Try V3 first (has fee field), then V2
            pool_info = self.discover_v3_pool(pool_address)
            if pool_info is None:
                pool_info = self.discover_v2_pool(pool_address)
        
        if pool_info is None:
            return None
        
        # Check if our token is involved
        token0 = pool_info['token0']
        token1 = pool_info['token1']
        
        if token0 == token_address:
            pool_info['denom_address'] = token1
            pool_info['token1_is_denom'] = True
        elif token1 == token_address:
            pool_info['denom_address'] = token0
            pool_info['token1_is_denom'] = False
        else:
            # Token not in this pool
            return None
        
        pool_info['token_address'] = token_address
        
        return pool_info
    
    def compute_v2_pool_address(self, denom_address: str) -> str:
        """
        Compute deterministic V2 pool address.
        
        Pool addresses in V2 are deterministic based on token pair.
        """        
        # Sort tokens
        token0, token1 = sorted([self.token_address, denom_address])
        
        # V2 uses CREATE2 with salt = keccak(token0, token1)
        salt = keccak(bytes.fromhex(token0[2:]) + bytes.fromhex(token1[2:]))
        
        # This is a simplified version - actual implementation would use CREATE2
        return f"0x{salt.hex()[:40]}"
    
    def compute_v3_pool_address(self, denom_address: str, fee: int = 3000) -> str:
        """
        Compute deterministic V3 pool address.
        
        Pool addresses in V3 are deterministic based on token pair and fee.
        """
        # Sort tokens
        token0, token1 = sorted([self.token_address, denom_address])
        
        # V3 includes fee in the salt
        fee_bytes = fee.to_bytes(3, 'big')
        salt_data = bytes.fromhex(token0[2:]) + bytes.fromhex(token1[2:]) + fee_bytes
        salt = keccak(salt_data)
        
        return f"0x{salt.hex()[:40]}"
    
    def get_or_create_v3_pool(self, denom_address: str, fee: int = 3000):
        """
        Get existing V3 pool or compute its address if it should exist.
        """
        pool_address = self.compute_v3_pool_address(denom_address, fee)
        
        if pool_address in self.pools:
            return self.pools[pool_address]
        
        # Check if pool exists on-chain and create if so
        # For now, return None - would need to call factory.getPool()
        return None
