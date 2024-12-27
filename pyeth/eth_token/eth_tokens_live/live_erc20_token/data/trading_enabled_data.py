from datetime import datetime, timezone
from typing import Optional, Tuple, Dict, Any
import pandas as pd


class TradingEnabledCalculator:
    """
    Calculates trading enabled information from price data.
    
    This calculator determines when trading was first enabled for a token by
    analyzing the first buy/sell transaction in the price data.
    """

    def __init__(self, price_df: pd.DataFrame):
        self.price_df = price_df

    def calculate_info(self) -> Dict[str, Any]:
        """Calculate all trading enabled information"""
        block, timestamp = self._get_first_trade_info()
        
        return {
            'trading_enabled_block': block,
            'trading_enabled_timestamp': timestamp,
            'trading_enabled_datetime': self._convert_timestamp_to_datetime(timestamp)
        }

    def _get_first_trade_info(self) -> Tuple[Optional[int], Optional[int]]:
        """Get block number and timestamp of first buy/sell transaction"""
        if not self.price_df.empty:
            filtered_df = self.price_df[
                (self.price_df['action'] == 'Buy') | 
                (self.price_df['action'] == 'Sell')
            ]
            if not filtered_df.empty:
                first_trade = filtered_df.iloc[0]
                return int(first_trade['block']), int(first_trade['timestamp'])
        return None, None

    def _convert_timestamp_to_datetime(self, timestamp: Optional[int]) -> Optional[datetime]:
        """Convert Unix timestamp to UTC datetime"""
        if timestamp is not None:
            return datetime.fromtimestamp(timestamp, tz=timezone.utc)
        return None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert trading enabled info to dictionary"""
        return {
            'trading_enabled_block': self.trading_enabled_block,
            'trading_enabled_timestamp': self.trading_enabled_timestamp,
            'trading_enabled_datetime': self.trading_enabled_datetime
        }
