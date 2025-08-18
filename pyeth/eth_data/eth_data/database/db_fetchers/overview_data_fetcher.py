"""
Overview Data Fetcher Module
--------------------------
This module provides methods to fetch overview and dashboard statistics from the database.
It includes functionality for retrieving aggregated metrics with time-based filtering.
"""

from sqlalchemy import text, func
from eth_data.database.eth_db_conn import get_db_session_maker
from datetime import datetime, timedelta
from typing import Dict, Optional


class OverviewDataFetcher:
    """Fetches and analyzes overview statistics from the database."""
    
    def __init__(self):
        """Initialize the OverviewDataFetcher with a database connection."""
        self.Session = get_db_session_maker(db='eth_db')
    
    def _get_time_filter(self, start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> tuple[str, dict]:
        """Helper method to generate time filter SQL and parameters."""
        time_filter = ""
        params = {}
        
        if start_date:
            time_filter += " AND b.block_timestamp >= :start_timestamp"
            params['start_timestamp'] = int(start_date.timestamp())
        if end_date:
            time_filter += " AND b.block_timestamp <= :end_timestamp"
            params['end_timestamp'] = int(end_date.timestamp())
            
        return time_filter, params
    
    def get_total_trades(self, start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> Dict:
        """Get total number of trades within a time range."""
        time_filter, params = self._get_time_filter(start_date, end_date)
        
        with self.Session() as session:
            query = text(f"""
                SELECT COUNT(*) as total_trades
                FROM eth_db.trades t
                JOIN eth_db.blocks b ON t.entry_block = b.block_number
                WHERE 1=1 {time_filter}
            """)
            result = session.execute(query, params).scalar_one()
            return {'total_trades': result}
    
    def get_active_addresses(self, start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> Dict:
        """Get number of active addresses within a time range."""
        time_filter, params = self._get_time_filter(start_date, end_date)
        
        with self.Session() as session:
            query = text(f"""
                SELECT COUNT(DISTINCT t.address_id) as active_addresses
                FROM eth_db.trades t
                JOIN eth_db.blocks b ON t.entry_block = b.block_number
                WHERE 1=1 {time_filter}
            """)
            result = session.execute(query, params).scalar_one()
            return {'active_addresses': result}
    
    def get_total_volume(self, start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> Dict:
        """Get total trading volume within a time range."""
        time_filter, params = self._get_time_filter(start_date, end_date)
        
        with self.Session() as session:
            query = text(f"""
                SELECT 
                    SUM(t.total_denom_spent) as total_volume_spent,
                    SUM(t.total_denom_received) as total_volume_received
                FROM eth_db.trades t
                JOIN eth_db.blocks b ON t.entry_block = b.block_number
                WHERE 1=1 {time_filter}
            """)
            result = session.execute(query, params).fetchone()
            total_volume = (result[0] or 0) + (result[1] or 0)
            return {
                'total_volume_spent': float(result[0] or 0),
                'total_volume_received': float(result[1] or 0),
                'total_volume': float(total_volume)
            }
    
    def get_total_profit(self, start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> Dict:
        """Get total realized profit and trade counts within a time range."""
        time_filter, params = self._get_time_filter(start_date, end_date)
        
        with self.Session() as session:
            query = text(f"""
                SELECT 
                    SUM(t.realized_profit) as total_profit,
                    COUNT(CASE WHEN t.realized_profit > 0 THEN 1 END) as profitable_trades,
                    COUNT(CASE WHEN t.realized_profit < 0 THEN 1 END) as loss_trades
                FROM eth_db.trades t
                JOIN eth_db.blocks b ON t.entry_block = b.block_number
                WHERE 1=1 {time_filter}
            """)
            result = session.execute(query, params).fetchone()
            return {
                'total_profit': float(result[0] or 0),
                'profitable_trades': result[1] or 0,
                'loss_trades': result[2] or 0
            }
    
    def get_recent_activity(self, limit: int = 10, start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> Dict:
        """Get recent trading activity within a time range."""
        time_filter, params = self._get_time_filter(start_date, end_date)
        params['limit'] = limit
        
        with self.Session() as session:
            query = text(f"""
                SELECT 
                    t.id,
                    t.address_id,
                    t.token_address,
                    t.realized_profit,
                    t.total_denom_spent,
                    t.total_denom_received,
                    t.entry_block,
                    a.address,
                    b.block_timestamp
                FROM eth_db.trades t
                LEFT JOIN eth_db.addresses a ON t.address_id = a.address_id
                JOIN eth_db.blocks b ON t.entry_block = b.block_number
                WHERE 1=1 {time_filter}
                ORDER BY t.entry_block DESC
                LIMIT :limit
            """)
            result = session.execute(query, params).fetchall()
            
            recent_trades = []
            for row in result:
                recent_trades.append({
                    'id': row[0],
                    'address_id': row[1],
                    'token_address': row[2],
                    'realized_profit': float(row[3] or 0),
                    'total_denom_spent': float(row[4] or 0),
                    'total_denom_received': float(row[5] or 0),
                    'entry_block': row[6],
                    'address': row[7],
                    'timestamp': row[8]
                })
            
            return {'recent_trades': recent_trades}
    
    def get_time_series_data(self, interval: str = 'day', start_date: Optional[datetime] = None, end_date: Optional[datetime] = None) -> Dict:
        """Get time series data for various metrics within a time range."""
        time_filter, params = self._get_time_filter(start_date, end_date)
        
        with self.Session() as session:
            query = text(f"""
                SELECT 
                    date_trunc(:interval, to_timestamp(b.block_timestamp)) as time_bucket,
                    COUNT(*) as trade_count,
                    COUNT(DISTINCT t.address_id) as unique_addresses,
                    SUM(t.total_denom_spent) as volume_spent,
                    SUM(t.total_denom_received) as volume_received,
                    SUM(t.realized_profit) as total_profit
                FROM eth_db.trades t
                JOIN eth_db.blocks b ON t.entry_block = b.block_number
                WHERE 1=1 {time_filter}
                GROUP BY time_bucket
                ORDER BY time_bucket ASC
            """)
            params['interval'] = interval
            result = session.execute(query, params).fetchall()
            
            time_series = []
            for row in result:
                time_series.append({
                    'timestamp': row[0],
                    'trade_count': row[1],
                    'unique_addresses': row[2],
                    'volume_spent': float(row[3] or 0),
                    'volume_received': float(row[4] or 0),
                    'total_profit': float(row[5] or 0)
                })
            
            return {'time_series': time_series} 