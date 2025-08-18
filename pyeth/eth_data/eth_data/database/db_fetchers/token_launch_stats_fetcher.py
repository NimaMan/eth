"""
Token Launch Data Fetcher Module

Objective:
---------
Provides specialized methods for retrieving token launch data, including
recent launches, launch trends, and launch-related analytics.

Key Features:
-----------
1. Recent Token Launches: Get tokens launched in the last N days
2. Launch Metadata: Trading enabled block, transaction, creator info
3. Time-based Filtering: Using timestamp-based queries for accuracy
4. Launch Analytics: Volume, risk patterns, launch trends
"""

from sqlalchemy import text, func
from eth_data.database.eth_db_conn import get_db_session_maker
from datetime import datetime, timedelta
import pandas as pd
from typing import Dict, List, Any


class TokenLaunchFetcher:
    
    def __init__(self):
        self.Session = get_db_session_maker(db='eth_db')
    
    def get_tokens_launched_last_n_days(self, days: int = 1, limit: int = 100, offset: int = 0):
        """
        Get tokens launched in the last N days using trading_enabled_txn as launch indicator.
        
        Args:
            days: Number of days to look back (default: 1)
            limit: Maximum number of tokens to return
            offset: Number of tokens to skip for pagination
            
        Returns:
            pandas.DataFrame: Token launch data with metadata
        """
        # Calculate timestamp threshold
        cutoff_time = datetime.now() - timedelta(days=days)
        cutoff_timestamp = int(cutoff_time.timestamp())
        
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.contract_address,
                    t.creator_address_id,
                    a_creator.address as creator_address,
                    t.is_scam,
                    t.scam_label,
                    t.creation_txn,
                    t.trading_enabled_txn,
                    tx_trading.block_number as trading_enabled_block,
                    b_trading.block_timestamp as trading_enabled_timestamp,
                    tx_creation.block_number as creation_block,
                    b_creation.block_timestamp as creation_timestamp
                FROM 
                    eth_db.tokens t
                LEFT JOIN 
                    eth_db.addresses a_creator ON t.creator_address_id = a_creator.address_id
                LEFT JOIN 
                    eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN 
                    eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                LEFT JOIN 
                    eth_db.transactions tx_creation ON t.creation_txn = tx_creation.tx_hash
                LEFT JOIN 
                    eth_db.blocks b_creation ON tx_creation.block_number = b_creation.block_number
                WHERE 
                    b_trading.block_timestamp >= :cutoff_timestamp
                    AND t.trading_enabled_txn IS NOT NULL
                ORDER BY 
                    b_trading.block_timestamp DESC
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(
                query, 
                {
                    "cutoff_timestamp": cutoff_timestamp,
                    "limit": limit,
                    "offset": offset
                }
            )
            
            columns = [
                'contract_address', 'creator_address_id', 'creator_address', 
                'is_scam', 'scam_label', 'creation_txn', 'trading_enabled_txn',
                'trading_enabled_block', 'trading_enabled_timestamp',
                'creation_block', 'creation_timestamp'
            ]
            
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            # Add helpful computed columns
            if not df.empty:
                # Convert timestamps to readable datetime
                df['trading_enabled_datetime'] = pd.to_datetime(df['trading_enabled_timestamp'], unit='s')
                df['creation_datetime'] = pd.to_datetime(df['creation_timestamp'], unit='s')
                
                # Add Etherscan links
                df['trading_enabled_etherscan_link'] = df['trading_enabled_txn'].apply(
                    lambda tx: f"https://etherscan.io/tx/{tx}" if tx else None
                )
                df['creation_etherscan_link'] = df['creation_txn'].apply(
                    lambda tx: f"https://etherscan.io/tx/{tx}" if tx else None
                )
                
                # Add time since launch
                now = datetime.now()
                df['hours_since_launch'] = (
                    now - df['trading_enabled_datetime']
                ).dt.total_seconds() / 3600
                
            return df
    
    def get_tokens_launched_date_range(self, start_date: str = None, end_date: str = None, limit: int = 100, offset: int = 0):
        """
        Get tokens launched within a specific date range using trading_enabled_txn as launch indicator.
        
        Args:
            start_date: Start date in YYYY-MM-DD format (inclusive)
            end_date: End date in YYYY-MM-DD format (inclusive)
            limit: Maximum number of tokens to return
            offset: Number of tokens to skip for pagination
            
        Returns:
            pandas.DataFrame: Token launch data with metadata
        """
        # Convert dates to timestamps
        start_timestamp = None
        end_timestamp = None
        
        if start_date:
            start_dt = datetime.strptime(start_date, '%Y-%m-%d')
            start_timestamp = int(start_dt.timestamp())
            
        if end_date:
            # End date should include the entire day, so add 24 hours - 1 second
            end_dt = datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1) - timedelta(seconds=1)
            end_timestamp = int(end_dt.timestamp())
        
        with self.Session() as session:
            # Build WHERE conditions
            where_conditions = ["t.trading_enabled_txn IS NOT NULL"]
            params = {"limit": limit, "offset": offset}
            
            if start_timestamp:
                where_conditions.append("b_trading.block_timestamp >= :start_timestamp")
                params["start_timestamp"] = start_timestamp
                
            if end_timestamp:
                where_conditions.append("b_trading.block_timestamp <= :end_timestamp")
                params["end_timestamp"] = end_timestamp
            
            where_clause = " AND ".join(where_conditions)
            
            query = text(f"""
                SELECT 
                    t.contract_address,
                    t.creator_address_id,
                    a_creator.address as creator_address,
                    t.is_scam,
                    t.scam_label,
                    t.creation_txn,
                    t.trading_enabled_txn,
                    tx_trading.block_number as trading_enabled_block,
                    b_trading.block_timestamp as trading_enabled_timestamp,
                    tx_creation.block_number as creation_block,
                    b_creation.block_timestamp as creation_timestamp
                FROM 
                    eth_db.tokens t
                LEFT JOIN 
                    eth_db.addresses a_creator ON t.creator_address_id = a_creator.address_id
                LEFT JOIN 
                    eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN 
                    eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                LEFT JOIN 
                    eth_db.transactions tx_creation ON t.creation_txn = tx_creation.tx_hash
                LEFT JOIN 
                    eth_db.blocks b_creation ON tx_creation.block_number = b_creation.block_number
                WHERE 
                    {where_clause}
                ORDER BY 
                    b_trading.block_timestamp DESC
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(query, params)
            
            columns = [
                'contract_address', 'creator_address_id', 'creator_address', 
                'is_scam', 'scam_label', 'creation_txn', 'trading_enabled_txn',
                'trading_enabled_block', 'trading_enabled_timestamp',
                'creation_block', 'creation_timestamp'
            ]
            
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            # Add helpful computed columns
            if not df.empty:
                # Convert timestamps to readable datetime
                df['trading_enabled_datetime'] = pd.to_datetime(df['trading_enabled_timestamp'], unit='s')
                df['creation_datetime'] = pd.to_datetime(df['creation_timestamp'], unit='s')
                
                # Add Etherscan links
                df['trading_enabled_etherscan_link'] = df['trading_enabled_txn'].apply(
                    lambda tx: f"https://etherscan.io/tx/{tx}" if tx else None
                )
                df['creation_etherscan_link'] = df['creation_txn'].apply(
                    lambda tx: f"https://etherscan.io/tx/{tx}" if tx else None
                )
                
                # Add time since launch
                now = datetime.now()
                df['hours_since_launch'] = (
                    now - df['trading_enabled_datetime']
                ).dt.total_seconds() / 3600
                
            return df
    
    def get_launch_count_date_range(self, start_date: str = None, end_date: str = None):
        """
        Get the count of tokens launched within a specific date range.
        
        Args:
            start_date: Start date in YYYY-MM-DD format
            end_date: End date in YYYY-MM-DD format
            
        Returns:
            int: Number of tokens launched
        """
        # Convert dates to timestamps
        start_timestamp = None
        end_timestamp = None
        
        if start_date:
            start_dt = datetime.strptime(start_date, '%Y-%m-%d')
            start_timestamp = int(start_dt.timestamp())
            
        if end_date:
            end_dt = datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1) - timedelta(seconds=1)
            end_timestamp = int(end_dt.timestamp())
        
        with self.Session() as session:
            # Build WHERE conditions
            where_conditions = ["t.trading_enabled_txn IS NOT NULL"]
            params = {}
            
            if start_timestamp:
                where_conditions.append("b_trading.block_timestamp >= :start_timestamp")
                params["start_timestamp"] = start_timestamp
                
            if end_timestamp:
                where_conditions.append("b_trading.block_timestamp <= :end_timestamp")
                params["end_timestamp"] = end_timestamp
            
            where_clause = " AND ".join(where_conditions)
            
            query = text(f"""
                SELECT COUNT(t.contract_address) 
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE {where_clause}
            """)
            
            count = session.execute(query, params).scalar_one_or_none()
            return count if count is not None else 0
    
    def get_launch_summary_date_range(self, start_date: str = None, end_date: str = None):
        """
        Get summary statistics for tokens launched within a specific date range.
        
        Args:
            start_date: Start date in YYYY-MM-DD format
            end_date: End date in YYYY-MM-DD format
            
        Returns:
            dict: Summary statistics including total, scam count, etc.
        """
        # Convert dates to timestamps
        start_timestamp = None
        end_timestamp = None
        
        if start_date:
            start_dt = datetime.strptime(start_date, '%Y-%m-%d')
            start_timestamp = int(start_dt.timestamp())
            
        if end_date:
            end_dt = datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1) - timedelta(seconds=1)
            end_timestamp = int(end_dt.timestamp())
        
        with self.Session() as session:
            # Build WHERE conditions
            where_conditions = ["t.trading_enabled_txn IS NOT NULL"]
            params = {}
            
            if start_timestamp:
                where_conditions.append("b_trading.block_timestamp >= :start_timestamp")
                params["start_timestamp"] = start_timestamp
                
            if end_timestamp:
                where_conditions.append("b_trading.block_timestamp <= :end_timestamp")
                params["end_timestamp"] = end_timestamp
            
            where_clause = " AND ".join(where_conditions)
            
            query = text(f"""
                SELECT 
                    COUNT(t.contract_address) as total_launches,
                    COUNT(CASE WHEN t.is_scam = true THEN 1 END) as scam_count,
                    COUNT(DISTINCT t.creator_address_id) as unique_creators,
                    MIN(b_trading.block_timestamp) as earliest_timestamp,
                    MAX(b_trading.block_timestamp) as latest_timestamp
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE {where_clause}
            """)
            
            result = session.execute(query, params)
            row = result.fetchone()
            
            if row:
                return {
                    "total_launches": row[0] or 0,
                    "scam_count": row[1] or 0,
                    "unique_creators": row[2] or 0,
                    "scam_percentage": (row[1] / row[0] * 100) if row[0] and row[0] > 0 else 0,
                    "earliest_launch": datetime.fromtimestamp(row[3]) if row[3] else None,
                    "latest_launch": datetime.fromtimestamp(row[4]) if row[4] else None,
                    "date_range": f"{start_date or 'start'} to {end_date or 'end'}"
                }
            
            return {
                "total_launches": 0,
                "scam_count": 0,
                "unique_creators": 0,
                "scam_percentage": 0,
                "earliest_launch": None,
                "latest_launch": None,
                "date_range": f"{start_date or 'start'} to {end_date or 'end'}"
            }
    
    def get_launch_count_last_n_days(self, days: int = 1):
        """
        Get the count of tokens launched in the last N days.
        
        Args:
            days: Number of days to look back
            
        Returns:
            int: Number of tokens launched
        """
        cutoff_time = datetime.now() - timedelta(days=days)
        cutoff_timestamp = int(cutoff_time.timestamp())
        
        with self.Session() as session:
            query = text("""
                SELECT COUNT(t.contract_address) 
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE 
                    b_trading.block_timestamp >= :cutoff_timestamp
                    AND t.trading_enabled_txn IS NOT NULL
            """)
            
            count = session.execute(query, {"cutoff_timestamp": cutoff_timestamp}).scalar_one_or_none()
            return count if count is not None else 0
    
    def get_launch_summary_last_n_days(self, days: int = 1):
        """
        Get summary statistics for tokens launched in the last N days.
        
        Args:
            days: Number of days to look back
            
        Returns:
            dict: Summary statistics including total, scam count, etc.
        """
        cutoff_time = datetime.now() - timedelta(days=days)
        cutoff_timestamp = int(cutoff_time.timestamp())
        
        with self.Session() as session:
            query = text("""
                SELECT 
                    COUNT(t.contract_address) as total_launches,
                    COUNT(CASE WHEN t.is_scam = true THEN 1 END) as scam_count,
                    COUNT(DISTINCT t.creator_address_id) as unique_creators,
                    MIN(b_trading.block_timestamp) as earliest_timestamp,
                    MAX(b_trading.block_timestamp) as latest_timestamp
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE 
                    b_trading.block_timestamp >= :cutoff_timestamp
                    AND t.trading_enabled_txn IS NOT NULL
            """)
            
            result = session.execute(query, {"cutoff_timestamp": cutoff_timestamp})
            row = result.fetchone()
            
            if row:
                return {
                    "total_launches": row[0] or 0,
                    "scam_count": row[1] or 0,
                    "unique_creators": row[2] or 0,
                    "scam_percentage": (row[1] / row[0] * 100) if row[0] and row[0] > 0 else 0,
                    "earliest_launch": datetime.fromtimestamp(row[3]) if row[3] else None,
                    "latest_launch": datetime.fromtimestamp(row[4]) if row[4] else None,
                    "days_analyzed": days
                }
            
            return {
                "total_launches": 0,
                "scam_count": 0,
                "unique_creators": 0,
                "scam_percentage": 0,
                "earliest_launch": None,
                "latest_launch": None,
                "days_analyzed": days
            }
    
    def get_scam_tokens_date_range(self, start_date: str = None, end_date: str = None, limit: int = 100) -> pd.DataFrame:
        """
        Get scam tokens launched within a specific date range.
        
        Args:
            start_date: Start date in YYYY-MM-DD format
            end_date: End date in YYYY-MM-DD format  
            limit: Maximum number of tokens to return
            
        Returns:
            pandas.DataFrame: Scam token data with metadata
        """
        # Convert dates to timestamps
        start_timestamp = None
        end_timestamp = None
        
        if start_date:
            start_dt = datetime.strptime(start_date, '%Y-%m-%d')
            start_timestamp = int(start_dt.timestamp())
            
        if end_date:
            end_dt = datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1) - timedelta(seconds=1)
            end_timestamp = int(end_dt.timestamp())
        
        with self.Session() as session:
            # Build WHERE conditions
            where_conditions = [
                "t.trading_enabled_txn IS NOT NULL",
                "t.is_scam = true"
            ]
            params = {"limit": limit}
            
            if start_timestamp:
                where_conditions.append("b_trading.block_timestamp >= :start_timestamp")
                params["start_timestamp"] = start_timestamp
                
            if end_timestamp:
                where_conditions.append("b_trading.block_timestamp <= :end_timestamp")
                params["end_timestamp"] = end_timestamp
            
            where_clause = " AND ".join(where_conditions)
            
            query = text(f"""
                SELECT 
                    t.contract_address,
                    t.creator_address_id,
                    a_creator.address as creator_address,
                    t.is_scam,
                    t.scam_label,
                    t.creation_txn,
                    t.trading_enabled_txn,
                    tx_trading.block_number as trading_enabled_block,
                    b_trading.block_timestamp as trading_enabled_timestamp,
                    tx_creation.block_number as creation_block,
                    b_creation.block_timestamp as creation_timestamp
                FROM 
                    eth_db.tokens t
                LEFT JOIN 
                    eth_db.addresses a_creator ON t.creator_address_id = a_creator.address_id
                LEFT JOIN 
                    eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN 
                    eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                LEFT JOIN 
                    eth_db.transactions tx_creation ON t.creation_txn = tx_creation.tx_hash
                LEFT JOIN 
                    eth_db.blocks b_creation ON tx_creation.block_number = b_creation.block_number
                WHERE {where_clause}
                ORDER BY 
                    b_trading.block_timestamp DESC
                LIMIT :limit
            """)
            
            result = session.execute(query, params)
            
            columns = [
                'contract_address', 'creator_address_id', 'creator_address', 
                'is_scam', 'scam_label', 'creation_txn', 'trading_enabled_txn',
                'trading_enabled_block', 'trading_enabled_timestamp',
                'creation_block', 'creation_timestamp'
            ]
            
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            # Add helpful computed columns
            if not df.empty:
                # Convert timestamps to readable datetime
                df['trading_enabled_datetime'] = pd.to_datetime(df['trading_enabled_timestamp'], unit='s')
                df['creation_datetime'] = pd.to_datetime(df['creation_timestamp'], unit='s')
                
                # Add Etherscan links
                df['trading_enabled_etherscan_link'] = df['trading_enabled_txn'].apply(
                    lambda tx: f"https://etherscan.io/tx/{tx}" if tx else None
                )
                df['creation_etherscan_link'] = df['creation_txn'].apply(
                    lambda tx: f"https://etherscan.io/tx/{tx}" if tx else None
                )
                
                # Add time since launch
                now = datetime.now()
                df['hours_since_launch'] = (
                    now - df['trading_enabled_datetime']
                ).dt.total_seconds() / 3600
                
            return df
    
    def get_scam_summary_date_range(self, start_date: str = None, end_date: str = None) -> Dict[str, Any]:
        """
        Get summary statistics for scam tokens within a date range.
        
        Args:
            start_date: Start date in YYYY-MM-DD format
            end_date: End date in YYYY-MM-DD format
            
        Returns:
            dict: Summary statistics
        """
        # Convert dates to timestamps
        start_timestamp = None
        end_timestamp = None
        
        if start_date:
            start_dt = datetime.strptime(start_date, '%Y-%m-%d')
            start_timestamp = int(start_dt.timestamp())
            
        if end_date:
            end_dt = datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1) - timedelta(seconds=1)
            end_timestamp = int(end_dt.timestamp())
        
        # Calculate 24h ago timestamp
        one_day_ago = datetime.now() - timedelta(days=1)
        one_day_ago_timestamp = int(one_day_ago.timestamp())
        
        with self.Session() as session:
            # Build WHERE conditions
            where_conditions = [
                "t.trading_enabled_txn IS NOT NULL",
                "t.is_scam = true"
            ]
            params = {}
            
            if start_timestamp:
                where_conditions.append("b_trading.block_timestamp >= :start_timestamp")
                params["start_timestamp"] = start_timestamp
                
            if end_timestamp:
                where_conditions.append("b_trading.block_timestamp <= :end_timestamp")
                params["end_timestamp"] = end_timestamp
            
            where_clause = " AND ".join(where_conditions)
            
            # Get overall stats
            query = text(f"""
                SELECT 
                    COUNT(t.contract_address) as total_scam_tokens,
                    COUNT(CASE WHEN b_trading.block_timestamp >= :one_day_ago THEN 1 END) as scams_last_24h,
                    COUNT(DISTINCT t.creator_address_id) as unique_scam_creators
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE {where_clause}
            """)
            
            params["one_day_ago"] = one_day_ago_timestamp
            result = session.execute(query, params)
            row = result.fetchone()
            
            # Get most common scam type
            query_scam_type = text(f"""
                SELECT scam_label, COUNT(*) as count
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE {where_clause}
                GROUP BY scam_label
                ORDER BY count DESC
                LIMIT 1
            """)
            
            # Remove one_day_ago param for scam type query
            params_scam_type = {k: v for k, v in params.items() if k != 'one_day_ago'}
            result_scam_type = session.execute(query_scam_type, params_scam_type)
            scam_type_row = result_scam_type.fetchone()
            
            # Calculate detection rate (scam tokens vs total tokens in period)
            query_total = text(f"""
                SELECT COUNT(t.contract_address) as total_tokens
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE t.trading_enabled_txn IS NOT NULL
                {"AND b_trading.block_timestamp >= :start_timestamp" if start_timestamp else ""}
                {"AND b_trading.block_timestamp <= :end_timestamp" if end_timestamp else ""}
            """)
            
            result_total = session.execute(query_total, params_scam_type)
            total_row = result_total.fetchone()
            
            detection_rate = 0
            if total_row and total_row[0] > 0:
                detection_rate = (row[0] / total_row[0]) * 100
            
            return {
                "total_scam_tokens": row[0] if row else 0,
                "scams_last_24h": row[1] if row else 0,
                "unique_scam_creators": row[2] if row else 0,
                "most_common_type": scam_type_row[0] if scam_type_row else "Unknown",
                "detection_rate": detection_rate
            }
    
    def get_scam_type_distribution(self, start_date: str = None, end_date: str = None) -> List[Dict[str, Any]]:
        """
        Get distribution of scam types within a date range.
        
        Args:
            start_date: Start date in YYYY-MM-DD format
            end_date: End date in YYYY-MM-DD format
            
        Returns:
            list: Distribution data for pie chart
        """
        # Convert dates to timestamps
        start_timestamp = None
        end_timestamp = None
        
        if start_date:
            start_dt = datetime.strptime(start_date, '%Y-%m-%d')
            start_timestamp = int(start_dt.timestamp())
            
        if end_date:
            end_dt = datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1) - timedelta(seconds=1)
            end_timestamp = int(end_dt.timestamp())
        
        with self.Session() as session:
            # Build WHERE conditions
            where_conditions = [
                "t.trading_enabled_txn IS NOT NULL",
                "t.is_scam = true"
            ]
            params = {}
            
            if start_timestamp:
                where_conditions.append("b_trading.block_timestamp >= :start_timestamp")
                params["start_timestamp"] = start_timestamp
                
            if end_timestamp:
                where_conditions.append("b_trading.block_timestamp <= :end_timestamp")
                params["end_timestamp"] = end_timestamp
            
            where_clause = " AND ".join(where_conditions)
            
            query = text(f"""
                SELECT 
                    COALESCE(scam_label, 'Unknown') as name,
                    COUNT(*) as value
                FROM eth_db.tokens t
                LEFT JOIN eth_db.transactions tx_trading ON t.trading_enabled_txn = tx_trading.tx_hash
                LEFT JOIN eth_db.blocks b_trading ON tx_trading.block_number = b_trading.block_number
                WHERE {where_clause}
                GROUP BY scam_label
                ORDER BY value DESC
            """)
            
            result = session.execute(query, params)
            
            distribution = []
            for row in result:
                distribution.append({
                    "name": row[0],
                    "value": row[1]
                })
            
            return distribution