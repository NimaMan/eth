"""
Token Lifetime Metrics
   - token_lifetime: Blocks, Hours from launch to last activity
   - token_status: Boolean indicating if token is still active after specific periods
"""

from typing import Optional, Dict, Any
import pandas as pd
import numpy as np
from datetime import datetime, timedelta, timezone


class TokenStatus:
    """
    Calculate token lifetime metrics

    The block diff threshold is set to 7200, the number of blocks in 24 hours at 12s per block
    """
    def __init__(self, live_token, 
                 inactivity_block_threshold: int=7200, 
                 min_daily_txns: int=5):
        self.live_token = live_token
        self.price_df = live_token.token_data.price_df.copy()
        self.inactivity_block_threshold = inactivity_block_threshold
        self.min_daily_txns = min_daily_txns
        self.remove_anomalies_from_price_df()
    
    def remove_anomalies_from_price_df(self):
        """Remove anomalies from the price dataframe by removing some of the latest txns
            - Check block gaps at the end of the dataset
            - If gaps exceed threshold, remove those periods
        """
        if self.price_df.empty or len(self.price_df) < self.min_daily_txns:
            return           
        # Calculate block differences
        block_diffs = self.price_df['block_number'].diff()
        
        # Look at the last 10 blocks to find inactivity
        latest_block_diffs = block_diffs.iloc[-10:]
        max_block_diff = latest_block_diffs.max()
        
        # If we find large gaps, trim the dataframe
        if max_block_diff > self.inactivity_block_threshold:
            # Find the last index where activity was normal
            last_active_idx = block_diffs[block_diffs <= self.inactivity_block_threshold].index[-1]
            self.price_df = self.price_df.iloc[:last_active_idx]        
    
    @property
    def token_status(self):
        """
        Calculate if token is still active based on recent activity.
        Returns False if:
        - No price data exists
        - Last activity was more than inactivity_threshold ago
        """
        if self.price_df.empty or self.token_data.trading_enabled_datetime is None:
            return "Creation"
        
        try:
            # Get current timestamp
            current_timestamp = int(datetime.now(timezone.utc).timestamp())
            
            # Get last activity timestamp
            last_activity_timestamp = self.price_df['timestamp'].iloc[-1]
            
            # Convert block threshold to time threshold (12s per block average)
            time_threshold = self.inactivity_block_threshold * 12  # seconds
            
            # Check if time since last activity exceeds threshold
            token_is_active = (current_timestamp - last_activity_timestamp) <= time_threshold
            if token_is_active:
                return "Active"
            return "Inactive"
        except (IndexError, KeyError):
            return "Other"

    def to_dict(self) -> Dict[str, Any]:
        """Convert metrics to dictionary"""
        return {
            'token_age_blocks': self.token_lifetime_blocks,
            'token_age_hours': self.token_lifetime_hours,
            'token_status': self.token_status,
            'last_activity_timestamp': self.last_activity_timestamp,
            'last_activity_datetime': self.last_activity_datetime,
            'last_activity_block': self.last_activity_block,
        }
        

