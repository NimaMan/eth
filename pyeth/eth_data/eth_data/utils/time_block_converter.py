"""
Simple Time-Block Converter
--------------------------

Provides utility functions to convert between Ethereum block numbers and timestamps
using simple approximations based on average block time.

This implementation uses a fixed estimate of 12 seconds per block and a reference point
to calculate conversions, without requiring complex database lookups.
"""

from datetime import datetime, timedelta
from typing import Optional, Tuple, Union


class TimeBlockConverter:
    """
    Simple utility class for converting between block numbers and timestamps.
    
    Uses a fixed average block time and simple math to estimate conversions.
    """
    # Ethereum's average block time (in seconds)
    AVG_BLOCK_TIME = 12
    # Reference point for Ethereum mainnet
    GENESIS_TIMESTAMP = 1438269973  # Timestamp of block 1 (July 30, 2015)

    def __init__(self, w3=None):
        self.w3 = w3
        latest_number, latest_timestamp = self._get_latest_block()
        self.reference_block = latest_number
        self.reference_timestamp = latest_timestamp
        
    def _get_latest_block(self) -> tuple:
        """
        Get the latest block info from Ethereum node, if available.
        Fall back to defaults if unavailable.
        
        Returns:
            tuple: (block_number, block_timestamp)
        """
        try:
            if self.w3 and self.w3.is_connected():
                # Get latest block using web3
                latest_block = self.w3.eth.get_block('latest')
                if latest_block:
                    return latest_block.number, latest_block.timestamp
        except Exception as e:
            print(f"Error getting latest block from web3: {e}")    
        return None, None
    
    def _estimate_block_by_time(self, timestamp: int) -> int:
        """
        Estimate block number for a timestamp based on genesis reference.
        
        Args:
            timestamp: Unix timestamp
            
        Returns:
            int: Estimated block number
        """
        seconds_since_genesis = timestamp - self.GENESIS_TIMESTAMP
        return max(1, int(seconds_since_genesis / self.AVG_BLOCK_TIME))
    
    def timestamp_to_block(self, timestamp: Union[int, datetime]) -> int:
        """
        Convert a timestamp to an estimated block number.
        
        Args:
            timestamp: Unix timestamp or datetime object
            
        Returns:
            int: Estimated block number
        """
        # Convert datetime to timestamp if needed
        if isinstance(timestamp, datetime):
            timestamp = int(timestamp.timestamp())
        
        # Calculate using simple proportion from reference point
        time_diff = timestamp - self.reference_timestamp
        block_diff = int(time_diff / self.AVG_BLOCK_TIME)
        estimated_block = max(1, self.reference_block + block_diff)
        
        return estimated_block
    
    def block_to_timestamp(self, block_number: int) -> int:
        """
        Convert a block number to an estimated timestamp.
        
        Args:
            block_number: Ethereum block number
            
        Returns:
            int: Estimated Unix timestamp
        """
        # Try to get exact timestamp from web3 if available
        if self.w3 and self.w3.is_connected():
            try:
                block = self.w3.eth.get_block(block_number)
                if block and 'timestamp' in block:
                    return block.timestamp
            except Exception:
                # Fall back to estimation if block not found or other error
                pass
                
        # Calculate using simple proportion from reference point
        block_diff = block_number - self.reference_block
        time_diff = block_diff * self.AVG_BLOCK_TIME
        estimated_timestamp = self.reference_timestamp + time_diff
        
        return int(estimated_timestamp)
    
    def time_range_to_block_range(self, start_time: Optional[Union[int, datetime]] = None, 
                                 end_time: Optional[Union[int, datetime]] = None) -> Tuple[Optional[int], Optional[int]]:
        """
        Convert a time range to a block range for filtering.
        
        Args:
            start_time: Start time (Unix timestamp or datetime, None for unbounded)
            end_time: End time (Unix timestamp or datetime, None for unbounded)
            
        Returns:
            Tuple[Optional[int], Optional[int]]: (start_block, end_block)
        """
        start_block = None if start_time is None else self.timestamp_to_block(start_time)
        end_block = None if end_time is None else self.timestamp_to_block(end_time)
        
        return start_block, end_block
    
    def timeframe_to_block_range(self, timeframe: str) -> Tuple[Optional[int], Optional[int]]:
        """
        Convert a named timeframe to a block range.
        
        Args:
            timeframe: Time period ('24h', '7d', '30d', 'all', etc.)
            
        Returns:
            Tuple[Optional[int], Optional[int]]: (start_block, end_block)
        """
        now = datetime.now()
        
        if timeframe == 'all':
            return None, None  # No bounds
        
        # Calculate start time based on timeframe
        start_time = None
        if timeframe == '24h':
            start_time = now - timedelta(hours=24)
        elif timeframe == '7d':
            start_time = now - timedelta(days=7)
        elif timeframe == '30d':
            start_time = now - timedelta(days=30)
        elif timeframe == '90d':
            start_time = now - timedelta(days=90)
        elif timeframe == '1y':
            start_time = now - timedelta(days=365)
        else:
            # Invalid timeframe
            print(f"Invalid timeframe: {timeframe}, using 'all'")
            return None, None
        
        # Convert to block range (end is None to use latest blocks)
        start_block = self.timestamp_to_block(start_time)
        end_block = self._get_latest_block()[0]
        
        return start_block, end_block 