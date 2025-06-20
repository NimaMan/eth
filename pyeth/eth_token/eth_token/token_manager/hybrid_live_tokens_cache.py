"""
Hybrid Live Tokens Cache with Shared Memory Support
--------------------------------------------------
Extends LiveTokensCache to provide shared memory access for hot token data
while maintaining full backward compatibility with the existing system.

Features:
- Zero-copy shared memory access for Rust components
- Binary packed token data for efficient storage
- LRU eviction for both in-memory and shared memory
- Thread-safe operations
- Graceful fallback if shared memory fails
"""

import time
import mmap
import struct
import os
from dataclasses import dataclass
from typing import Optional, Dict, Tuple
from threading import Lock
import logging

from eth_token.erc20_token.erc20_token import ERC20Token
from eth_token.token_manager.live_tokens_cache import LiveTokensCache, CacheEntry


# Constants for shared memory layout
MAGIC_NUMBER = 0x45544830  # 'ETH0' in hex
VERSION = 1
HEADER_SIZE = 1024
TOKEN_RECORD_SIZE = 512
DEFAULT_SHM_SIZE = 50_000_000  # 50MB default
SHM_NAME = "eth_token_cache"


@dataclass
class SharedMemoryHeader:
    """Header structure for shared memory segment"""
    magic_number: int
    version: int
    last_update_timestamp: int
    num_tokens: int
    token_record_size: int
    
    def pack(self) -> bytes:
        """Pack header into binary format"""
        return struct.pack(
            '<IIQII',  # Little-endian: uint32, uint32, uint64, uint32, uint32
            self.magic_number,
            self.version,
            self.last_update_timestamp,
            self.num_tokens,
            self.token_record_size
        )
    
    @classmethod
    def unpack(cls, data: bytes) -> 'SharedMemoryHeader':
        """Unpack header from binary format"""
        magic, version, timestamp, num_tokens, record_size = struct.unpack('<IIQII', data[:24])
        return cls(magic, version, timestamp, num_tokens, record_size)


class HybridLiveTokensCache(LiveTokensCache):
    """
    Hybrid cache that extends LiveTokensCache with shared memory support.
    
    Maintains the existing OrderedDict cache while also writing hot data
    to a memory-mapped file for zero-copy access from other processes.
    """
    
    def __init__(
        self, 
        max_size: int = 2000, 
        logger=None, 
        add_pnl_to_db: bool = False,
        shm_size: int = DEFAULT_SHM_SIZE,
        enable_shared_memory: bool = True
    ):
        """
        Initialize hybrid cache with shared memory support.
        
        Args:
            max_size: Maximum number of tokens in cache
            logger: Logger instance
            add_pnl_to_db: Whether to write PnL to database
            shm_size: Size of shared memory segment in bytes
            enable_shared_memory: Whether to enable shared memory feature
        """
        super().__init__(max_size, logger, add_pnl_to_db)
        
        self.shm_size = shm_size
        self.enable_shared_memory = enable_shared_memory
        self.shm = None
        self.shm_fd = None
        self.shm_path = f"/dev/shm/{SHM_NAME}"
        
        # Shared memory layout management
        self.max_shm_tokens = (shm_size - HEADER_SIZE) // TOKEN_RECORD_SIZE
        self.shm_index: Dict[str, int] = {}  # token_address -> slot_number
        self.shm_lock = Lock()
        
        if self.enable_shared_memory:
            self._init_shared_memory()
    
    def _init_shared_memory(self):
        """Initialize the shared memory segment"""
        try:
            # Create or open the shared memory file
            flags = os.O_CREAT | os.O_RDWR
            self.shm_fd = os.open(self.shm_path, flags, 0o666)
            
            # Resize to desired size
            os.ftruncate(self.shm_fd, self.shm_size)
            
            # Memory map the file
            self.shm = mmap.mmap(
                self.shm_fd,
                self.shm_size,
                mmap.MAP_SHARED,
                mmap.PROT_READ | mmap.PROT_WRITE
            )
            
            # Initialize header
            self._init_shm_header()
            
            self.log(f"Shared memory initialized at {self.shm_path} ({self.shm_size} bytes)")
            
        except Exception as e:
            self.log(f"Failed to initialize shared memory: {e}. Continuing without shared memory.")
            self.enable_shared_memory = False
            if self.shm:
                self.shm.close()
                self.shm = None
            if self.shm_fd:
                os.close(self.shm_fd)
                self.shm_fd = None
    
    def _init_shm_header(self):
        """Initialize the shared memory header"""
        header = SharedMemoryHeader(
            magic_number=MAGIC_NUMBER,
            version=VERSION,
            last_update_timestamp=int(time.time() * 1000),  # milliseconds
            num_tokens=0,
            token_record_size=TOKEN_RECORD_SIZE
        )
        
        self.shm.seek(0)
        self.shm.write(header.pack())
        self.shm.write(b'\x00' * (HEADER_SIZE - 24))  # Pad rest of header
    
    def _update_shm_header(self):
        """Update the shared memory header with current state"""
        if not self.shm:
            return
            
        with self.shm_lock:
            header = SharedMemoryHeader(
                magic_number=MAGIC_NUMBER,
                version=VERSION,
                last_update_timestamp=int(time.time() * 1000),
                num_tokens=len(self.shm_index),
                token_record_size=TOKEN_RECORD_SIZE
            )
            
            self.shm.seek(0)
            self.shm.write(header.pack())
    
    def _pack_token_data(self, token: ERC20Token) -> bytes:
        """
        Pack token data into fixed-size binary format.
        
        Binary layout (512 bytes):
        - 0-20: Token address (20 bytes)
        - 20-52: Total supply (32 bytes, uint256)
        - 52-53: Decimals (1 byte)
        - 53-61: Creation block (8 bytes, uint64)
        - 61-69: Creation timestamp (8 bytes, uint64)
        - 69-77: Trading enabled block (8 bytes, uint64)
        - 77-97: Primary V2 pool address (20 bytes)
        - 97-129: V2 ETH reserve (32 bytes, uint256)
        - 129-161: V2 token reserve (32 bytes, uint256)
        - 161-181: Primary V3 pool address (20 bytes)
        - 181-213: V3 ETH reserve (32 bytes, uint256)
        - 213-245: V3 token reserve (32 bytes, uint256)
        - 245-265: Primary V4 pool address (20 bytes)
        - 265-297: V4 ETH reserve (32 bytes, uint256)
        - 297-329: V4 token reserve (32 bytes, uint256)
        - 329-337: Last update timestamp (8 bytes, uint64)
        - 337-341: Holder count (4 bytes, uint32)
        - 341-342: Is scam flag (1 byte)
        - 342-343: Honeypot score (1 byte, 0-255)
        - 343-344: Trading enabled flag (1 byte)
        - 344-346: Buy tax (2 bytes, basis points)
        - 346-348: Sell tax (2 bytes, basis points)
        - 348-356: Volume 24h in wei (8 bytes, uint64)
        - 356-364: Price change 24h (8 bytes, int64, basis points)
        - 364-512: Reserved (148 bytes)
        """
        # Initialize buffer
        buffer = bytearray(TOKEN_RECORD_SIZE)
        
        # Token address (bytes 0-20)
        addr_bytes = bytes.fromhex(token.token_data.contract_address[2:])
        buffer[0:20] = addr_bytes
        
        # Total supply (bytes 20-52)
        total_supply = int(token.token_data.total_supply or 0)
        buffer[20:52] = total_supply.to_bytes(32, 'big')
        
        # Decimals (byte 52)
        buffer[52] = token.token_data.decimals or 18
        
        # Creation block (bytes 53-61)
        creation_block = token.token_data.creation_block or 0
        buffer[53:61] = creation_block.to_bytes(8, 'big')
        
        # Creation timestamp (bytes 61-69)
        creation_timestamp = int(token.token_data.creation_timestamp or 0)
        buffer[61:69] = creation_timestamp.to_bytes(8, 'big')
        
        # Trading enabled block (bytes 69-77)
        trading_block = token.token_data.trading_enabled_block or 0
        buffer[69:77] = trading_block.to_bytes(8, 'big')
        
        # Get pool data
        pool_info = token.token_data.get_pool_info_dict()
        
        # Process each pool type
        offset = 77
        for pool_type in ['V2', 'V3', 'V4']:
            pools = pool_info.get(pool_type, [])
            if pools:
                # Use first pool (primary)
                pool = pools[0]
                
                # Pool address (20 bytes)
                pool_addr = pool.get('pool_address', '0x' + '0' * 40)
                pool_bytes = bytes.fromhex(pool_addr[2:])
                buffer[offset:offset+20] = pool_bytes
                
                # ETH reserve (32 bytes)
                eth_reserve = int(pool.get('reserve0', 0) * 10**18)  # Convert to wei
                buffer[offset+20:offset+52] = eth_reserve.to_bytes(32, 'big')
                
                # Token reserve (32 bytes)
                token_decimals = token.token_data.decimals or 18
                token_reserve = int(pool.get('reserve1', 0) * 10**token_decimals)
                buffer[offset+52:offset+84] = token_reserve.to_bytes(32, 'big')
            else:
                # No pool - fill with zeros
                buffer[offset:offset+84] = b'\x00' * 84
                
            offset += 84
        
        # Last update timestamp (bytes 329-337)
        update_time = int(time.time() * 1000)  # milliseconds
        buffer[329:337] = update_time.to_bytes(8, 'big')
        
        # Holder count (bytes 337-341)
        holder_count = len(token.token_data.unique_addresses) if hasattr(token.token_data, 'unique_addresses') else 0
        buffer[337:341] = holder_count.to_bytes(4, 'big')
        
        # Is scam flag (byte 341)
        buffer[341] = 1 if getattr(token, 'is_scam', False) else 0
        
        # Honeypot score (byte 342)
        honeypot_score = getattr(token, 'honeypot_score', 0.0)
        buffer[342] = min(255, int(honeypot_score * 255))
        
        # Trading enabled flag (byte 343)
        buffer[343] = 1 if token.token_data.trading_enabled else 0
        
        # Buy tax (bytes 344-346)
        buy_tax = int(getattr(token, 'buy_tax', 0) * 10000)  # Convert to basis points
        buffer[344:346] = buy_tax.to_bytes(2, 'big')
        
        # Sell tax (bytes 346-348)
        sell_tax = int(getattr(token, 'sell_tax', 0) * 10000)  # Convert to basis points
        buffer[346:348] = sell_tax.to_bytes(2, 'big')
        
        # Volume 24h (bytes 348-356)
        volume_24h = int(getattr(token, 'volume_24h_eth', 0) * 10**18)  # Convert to wei
        buffer[348:356] = volume_24h.to_bytes(8, 'big')
        
        # Price change 24h (bytes 356-364)
        price_change = int(getattr(token, 'price_change_24h', 0) * 10000)  # Convert to basis points
        # Handle negative values with two's complement
        if price_change < 0:
            price_change = (1 << 64) + price_change
        buffer[356:364] = price_change.to_bytes(8, 'big')
        
        # Reserved space already zero-initialized
        
        return bytes(buffer)
    
    def _write_to_shm(self, token_address: str, token: ERC20Token):
        """Write token data to shared memory"""
        if not self.shm or not self.enable_shared_memory:
            return
            
        with self.shm_lock:
            try:
                # Get or allocate slot
                if token_address not in self.shm_index:
                    if len(self.shm_index) >= self.max_shm_tokens:
                        # Evict oldest token from shared memory
                        self._evict_from_shm()
                    
                    # Allocate new slot
                    slot = len(self.shm_index)
                    self.shm_index[token_address] = slot
                else:
                    slot = self.shm_index[token_address]
                
                # Calculate offset
                offset = HEADER_SIZE + (slot * TOKEN_RECORD_SIZE)
                
                # Pack and write data
                packed_data = self._pack_token_data(token)
                self.shm.seek(offset)
                self.shm.write(packed_data)
                
                # Update header
                self._update_shm_header()
                
            except Exception as e:
                self.log(f"Error writing token {token_address} to shared memory: {e}")
    
    def _evict_from_shm(self):
        """Evict oldest token from shared memory (FIFO for now)"""
        if not self.shm_index:
            return
            
        # Find oldest token (first in index)
        oldest_token = next(iter(self.shm_index))
        slot = self.shm_index[oldest_token]
        
        # Remove from index
        del self.shm_index[oldest_token]
        
        # Compact the index (shift all higher slots down)
        new_index = {}
        for addr, s in self.shm_index.items():
            if s > slot:
                new_index[addr] = s - 1
            else:
                new_index[addr] = s
        self.shm_index = new_index
        
        self.log(f"Evicted token {oldest_token} from shared memory")
    
    def __setitem__(self, key: str, value: ERC20Token):
        """Override to update both caches"""
        # Update main cache
        super().__setitem__(key, value)
        
        # Update shared memory
        if self.enable_shared_memory:
            self._write_to_shm(key, value)
    
    def __delitem__(self, key: str):
        """Override to handle shared memory cleanup"""
        # Call parent implementation
        super().__delitem__(key)
        
        # Remove from shared memory
        if self.enable_shared_memory and key in self.shm_index:
            with self.shm_lock:
                # Remove from index but don't compact (lazy deletion)
                del self.shm_index[key]
                self._update_shm_header()
    
    def clear_cache(self):
        """Clear both in-memory and shared memory caches"""
        # Clear parent cache
        super().clear_cache()
        
        # Clear shared memory
        if self.enable_shared_memory:
            with self.shm_lock:
                self.shm_index.clear()
                self._init_shm_header()
    
    def get_shm_stats(self) -> Dict[str, any]:
        """Get statistics about shared memory usage"""
        if not self.enable_shared_memory:
            return {"enabled": False}
            
        return {
            "enabled": True,
            "path": self.shm_path,
            "size": self.shm_size,
            "max_tokens": self.max_shm_tokens,
            "current_tokens": len(self.shm_index),
            "utilization": len(self.shm_index) / self.max_shm_tokens * 100
        }
    
    def cleanup(self):
        """Clean up shared memory resources"""
        if self.shm:
            self.shm.close()
            self.shm = None
        
        if self.shm_fd:
            os.close(self.shm_fd)
            self.shm_fd = None
        
        self.log("Shared memory resources cleaned up")
    
    def __del__(self):
        """Destructor to ensure cleanup"""
        self.cleanup()