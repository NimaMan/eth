"""
Base class for entity queries using PyReth.

This module provides the foundation for all entity-specific queries,
using PyReth's module-level singleton accessor and providing common
utilities like time conversion and error handling.
"""

from typing import Optional, Tuple
from datetime import datetime, timedelta
from enum import Enum
from pyreth import chain_query


class TimePeriod(Enum):
    """Standard time periods for queries."""
    HOUR_1 = "1h"
    HOUR_4 = "4h"
    HOUR_24 = "24h"
    DAY_7 = "7d"
    DAY_30 = "30d"
    # Aliases for compatibility
    HOURLY = "1h"
    DAILY = "24h"
    WEEKLY = "7d"
    MONTHLY = "30d"
    
    def to_hours(self) -> int:
        """Convert period to hours."""
        mapping = {
            "1h": 1,
            "4h": 4,
            "24h": 24,
            "7d": 168,
            "30d": 720,
        }
        return mapping[self.value]
    
    def to_timedelta(self) -> timedelta:
        """Convert to timedelta object."""
        return timedelta(hours=self.to_hours())
    
    def blocks_per_period(self) -> int:
        """Get approximate blocks per period (12 sec per block)."""
        return self.to_hours() * 300  # 300 blocks per hour


class BaseEntityQuery:
    """
    Base class for all entity queries using PyReth singleton.
    
    Provides:
    - Singleton PyReth chain query access
    - Time to block conversion
    - Common error handling
    - Caching utilities
    """
    
    # Class-level PyReth singleton
    _chain_query = None

    def __init__(self):
        """Initialize from the pyreth module-level singleton accessor."""
        if BaseEntityQuery._chain_query is None:
            BaseEntityQuery._chain_query = chain_query()
        self.query = BaseEntityQuery._chain_query
    
    def convert_time_to_blocks(
        self, 
        hours_back: int,
        from_block: Optional[int] = None
    ) -> Tuple[int, int]:
        """
        Convert a time range in hours to block numbers.
        
        Args:
            hours_back: Number of hours to go back
            from_block: Optional end block (default: latest)
            
        Returns:
            Tuple of (start_block, end_block)
        """
        from datetime import timezone
        
        # Get current time and calculate start time
        now = datetime.now(timezone.utc)
        start_time = now - timedelta(hours=hours_back)
        
        # Convert to ISO format for PyReth
        start_iso = start_time.isoformat().replace('+00:00', 'Z')
        end_iso = now.isoformat().replace('+00:00', 'Z')
        
        # Get block range
        start_block, end_block = self.query.get_blocks_for_time_range(
            start_iso, end_iso
        )
        
        # Override end block if specified
        if from_block:
            end_block = from_block
            
        return start_block, end_block
    
    def get_latest_block(self) -> int:
        """Get the latest block number."""
        return self.query.get_latest_block()
    
    def format_percentage(self, value: float, decimals: int = 2) -> str:
        """Format a float as percentage string."""
        return f"{value:.{decimals}f}%"
    
    def format_usd(self, value: float, decimals: int = 0) -> str:
        """Format a float as USD string."""
        if abs(value) >= 1_000_000_000:
            return f"${value/1_000_000_000:.1f}B"
        elif abs(value) >= 1_000_000:
            return f"${value/1_000_000:.1f}M"
        elif abs(value) >= 1_000:
            return f"${value/1_000:.1f}K"
        else:
            return f"${value:.{decimals}f}"
    
    def format_eth(self, value: float, decimals: int = 2) -> str:
        """Format a float as ETH string."""
        if abs(value) >= 1_000_000:
            return f"{value/1_000_000:.1f}M ETH"
        elif abs(value) >= 1_000:
            return f"{value/1_000:.1f}K ETH"
        else:
            return f"{value:.{decimals}f} ETH"
