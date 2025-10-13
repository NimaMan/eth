"""
Address Data Fetcher Module
--------------------------
This module provides methods to fetch and analyze address-related data from the database.
It includes functionality for retrieving address profiles, trading metrics, and network relationships.
"""

from requests import Session
from sqlalchemy import create_engine, text, select
from eth_data.database.eth_db_conn import get_db_session_maker
import pandas as pd
from datetime import datetime, timedelta
from typing import List, Dict, Optional, Union
from eth_data.database.eth_db_conn import get_db_engine
from eth_data.database.schema.eth_db_data_models import Address


class AddressDataFetcher:
    """Fetches and analyzes address-related data from the database."""
    
    def __init__(self):
        """Initialize the AddressDataFetcher with a database connection."""
        self.engine = get_db_engine()
        self.Session = get_db_session_maker(db='eth_db')
        self._address_cache = {}  # Cache for address lookups
    
    def _get_address_id(self, session: Session, address: str) -> Optional[int]:
        """Helper method to get address_id from address string."""
        stmt = text("SELECT address_id FROM eth_db.addresses WHERE address = :address")
        result = session.execute(stmt, {"address": address}).scalar_one_or_none()
        return result
    
    def get_address_blocks(self, address, start_block=None, end_block=None):
        """
        Get blocks where the address has activity.
        
        Args:
            address: Address string
            start_block: Optional starting block number
            end_block: Optional ending block number
            
        Returns:
            list: Block numbers where the address has activity
        """
        with self.Session() as session:
            # First get the address_id
            address_id = self._get_address_id(session, address)
            if not address_id:
                return []

            query = text("""
                SELECT 
                    t.block_number,
                    COUNT(DISTINCT t.tx_hash) as tx_count
                FROM 
                    eth_db.transactions t
                JOIN 
                    eth_db.tx_participants tp ON t.tx_hash = tp.tx_hash
                WHERE 
                    tp.address_id = :address_id
                    AND (:start_block IS NULL OR t.block_number >= :start_block)
                    AND (:end_block IS NULL OR t.block_number <= :end_block)
                GROUP BY 
                    t.block_number
                ORDER BY 
                    t.block_number ASC
            """)

            result = session.execute(
                query, 
                {
                    "address_id": address_id,
                    "start_block": start_block,
                    "end_block": end_block
                }
            )
            
            address_blocks = [row[0] for row in result.fetchall()]
            return address_blocks
    
    def get_address_transactions(self, address, start_block=None, end_block=None, limit=100, offset=0):
        """
        Get transactions involving the address.
        
        Args:
            address: Address string
            start_block: Optional starting block number
            end_block: Optional ending block number
            limit: Maximum number of transactions to return
            offset: Number of transactions to skip
            
        Returns:
            list: Transaction details involving the address
        """
        with self.Session() as session:
            # First get the address_id
            address_id = self._get_address_id(session, address)
            if not address_id:
                return []

            query = text("""
                SELECT DISTINCT
                    t.tx_hash,
                    t.block_number,
                    t.from_address_id,
                    t.to_address_id,
                    t.value,
                    t.status,
                    bl.block_timestamp
                FROM 
                    eth_db.transactions t
                JOIN 
                    eth_db.tx_participants tp ON t.tx_hash = tp.tx_hash
                JOIN
                    eth_db.blocks bl ON t.block_number = bl.block_number
                WHERE 
                    tp.address_id = :address_id
                    AND (:start_block IS NULL OR t.block_number >= :start_block)
                    AND (:end_block IS NULL OR t.block_number <= :end_block)
                ORDER BY 
                    bl.block_timestamp DESC
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(
                query, 
                {
                    "address_id": address_id,
                    "start_block": start_block,
                    "end_block": end_block,
                    "limit": limit,
                    "offset": offset
                }
            )
            
            # Fetch all rows first since we need to iterate twice
            rows = result.fetchall()
            
            # Get address strings for from/to addresses
            address_map = {}
            for row in rows:
                if row[2]: address_map[row[2]] = None  # from_address_id
                if row[3]: address_map[row[3]] = None  # to_address_id
            
            if address_map:
                stmt = select(Address.address_id, Address.address).where(Address.address_id.in_(address_map.keys()))
                for addr_id, addr_str in session.execute(stmt):
                    address_map[addr_id] = addr_str
            
            transactions = []
            for row in rows:
                transactions.append({
                    "tx_hash": row[0],
                    "block_number": row[1],
                    "from_address": address_map.get(row[2]),
                    "to_address": address_map.get(row[3]),
                    "value": row[4],
                    "status": row[5],
                    "timestamp": datetime.fromtimestamp(row[6]) if row[6] else None
                })
            
        return transactions
            
    def get_address_trades(self, address, limit=100, offset=0, timeframe='all'):
        """
        Get trades involving the address - optimized for speed.
        
        Args:
            address: Address string
            limit: Maximum number of trades to return
            offset: Number of trades to skip
            timeframe: 'day', 'week', 'month', 'year', or 'all' (Currently ignored due to blocks table issue)
            
        Returns:
            pandas.DataFrame: Trade data for the address
        """
        # Get address_id (from cache or DB)
        if address in self._address_cache:
            address_id = self._address_cache[address]
        else:
            with self.engine.connect() as conn:
                addr_query = text("SELECT address_id FROM eth_db.addresses WHERE address = :address")
                address_id = conn.execute(addr_query, {"address": address}).scalar()
                if address_id:
                    self._address_cache[address] = address_id
        
        if not address_id:
            return pd.DataFrame()

        # Timeframe filtering is removed for now as it depended on the empty blocks table
        # TODO: Re-implement timeframe filter if blocks table is populated or find alternative
        
        # Simplified query without blocks join
        query = text(f"""
            SELECT 
                t.id,
                t.token_address,
                t.currency,
                t.entry_block,
                t.latest_block,
                t.total_denom_spent,
                t.total_denom_received, 
                t.denom_received_spent_ratio, 
                t.bribe_amount,
                t.realized_profit, 
                t.unrealized_profit, 
                t.num_buys, 
                t.num_sells, 
                t.token_holdings_ratio,
                t.token_sell_buy_ratio, 
                t.agg_denom_balance, 
                t.agg_token_balance,
                t.tx_fee
            FROM 
                eth_db.trades t
            WHERE 
                t.address_id = :address_id
            ORDER BY 
                t.entry_block DESC
            LIMIT :limit OFFSET :offset
        """)

        # Execute query with direct engine connection
        with self.engine.connect() as conn:
            params = {
                "address_id": address_id,
                "limit": limit,
                "offset": offset
            }
            
            # Removed start_timestamp parameter as time filtering is disabled
                
            result = conn.execute(query, params)
            
            # Updated column list (removed block_timestamp)
            columns = ['id', 'token_address', 'currency', 'entry_block', 'latest_block',
                      'total_denom_spent', 'total_denom_received', 'denom_received_spent_ratio',
                      'bribe_amount', 'realized_profit', 'unrealized_profit', 'num_buys', 
                      'num_sells', 'token_holdings_ratio', 'token_sell_buy_ratio', 
                      'agg_denom_balance', 'agg_token_balance', 'tx_fee']
            
            # Create DataFrame directly from result
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            # Timestamp conversion removed
                
            return df
    
    def get_address_metadata(self, address: str) -> Dict:
        """
        Get comprehensive profile for an address including trading metrics and statistics.
        
        Args:
            address: The Ethereum address string
            
        Returns:
            Dict containing address profile data
        """
        with self.Session() as session:
            # First get the address_id
            address_id = self._get_address_id(session, address)
            if not address_id:
                return None

            query = text("""
            SELECT *
            FROM 
                eth_db.addresses a
            WHERE 
                a.address_id = :address_id
            """)
            
            result = session.execute(query, {"address_id": address_id})
            row = result.fetchone()
            
            if row:
                return  row._asdict()
            return None
    
    def get_top_traders_by_profit(self, limit: int = 10, timeframe: str = 'all') -> List[Dict]:
        """
        Get top traders by total realized profit.
        
        Args:
            limit: Maximum number of traders to return
            timeframe: Time period to consider ('day', 'week', 'month', 'year', 'all')
            
        Returns:
            List of trader dictionaries sorted by profit
        """
        # Calculate start timestamp based on timeframe
        start_timestamp = self._get_start_timestamp(timeframe)
        
        query = text("""
        SELECT 
            a.address,
            a.is_contract,
            a.total_realized_profit,
            a.total_volume,
            a.total_erc20_trades,
            a.scam_ratio,
            a.first_seen,
            a.last_seen,
            a.total_denom_balance,
            a.cluster_label
        FROM 
            eth_db.addresses a
        WHERE a.last_seen >= :start_timestamp
        ORDER BY 
            a.total_realized_profit DESC
        LIMIT :limit
        """)
        
        with self.Session() as session:
            result = session.execute(
                query,
                {
                    "start_timestamp": start_timestamp,
                    "limit": limit
                }
            )
            return [dict(row) for row in result]
    
    def get_top_traders_by_volume(self, limit: int = 10, timeframe: str = 'all') -> List[Dict]:
        """
        Get top traders by trading volume.
        
        Args:
            limit: Maximum number of traders to return
            timeframe: Time period to consider ('day', 'week', 'month', 'year', 'all')
            
        Returns:
            List of trader dictionaries sorted by volume
        """
        start_timestamp = self._get_start_timestamp(timeframe)
        
        query = text("""
        SELECT 
            a.address,
            a.is_contract,
            a.total_volume,
            a.total_realized_profit,
            a.total_erc20_trades,
            a.scam_ratio,
            a.first_seen,
            a.last_seen,
            a.total_denom_balance,
            a.cluster_label
        FROM 
            eth_db.addresses a
        WHERE a.last_seen >= :start_timestamp
        ORDER BY 
            a.total_volume DESC
        LIMIT :limit
        """)
        
        with self.Session() as session:
            result = session.execute(
                query,
                {
                    "start_timestamp": start_timestamp,
                    "limit": limit
                }
            )
            return [dict(row) for row in result]
    
    def get_top_traders_by_roi(self, limit: int = 10, min_volume: float = 1.0, timeframe: str = 'all') -> List[Dict]:
        """
        Get top traders by return on investment (ROI).
        
        Args:
            limit: Maximum number of traders to return
            min_volume: Minimum trading volume required
            timeframe: Time period to consider ('day', 'week', 'month', 'year', 'all')
            
        Returns:
            List of trader dictionaries sorted by ROI
        """
        start_timestamp = self._get_start_timestamp(timeframe)
        
        query = text("""
        SELECT 
            a.address,
            a.is_contract,
            a.total_realized_profit,
            a.total_volume,
            a.total_realized_profit / NULLIF(a.total_volume, 0) as roi,
            a.total_erc20_trades,
            a.scam_ratio,
            a.first_seen,
            a.last_seen,
            a.total_denom_balance,
            a.cluster_label
        FROM 
            eth_db.addresses a
        WHERE 
            a.total_volume >= :min_volume
            AND a.total_realized_profit > 0
            AND a.last_seen >= :start_timestamp
        ORDER BY 
            roi DESC
        LIMIT :limit
        """)
        
        with self.Session() as session:
            result = session.execute(
                query,
                {
                    "min_volume": min_volume,
                    "start_timestamp": start_timestamp,
                    "limit": limit
                }
            )
            return [dict(row) for row in result]
    
    def get_address_performance_time_series(self, interval: str = 'day', timeframe: str = 'day') -> Dict[str, List]:
        """Get time series data for address performance metrics.
        
        Args:
            interval: Time interval for aggregation ('hour', 'day', 'week', 'month')
            timeframe: Time period to analyze ('day', 'week', 'month', 'year', 'all')
            
        Returns:
            Dict containing lists of timestamps and metrics
        """
        start_timestamp = self._get_start_timestamp(timeframe)
        
        # Define interval in PostgreSQL date_trunc format
        interval_map = {
            'hour': 'hour',
            'day': 'day',
            'week': 'week',
            'month': 'month'
        }
        pg_interval = interval_map.get(interval, 'day')
        
        query = text(f"""
            SELECT 
                date_trunc('{pg_interval}', to_timestamp(b.block_timestamp)) as time_bucket,
                SUM(t.realized_profit) as total_profit,
                SUM(t.total_denom_spent) as total_volume,
                CASE 
                    WHEN SUM(t.total_denom_spent) > 0 
                    THEN SUM(t.realized_profit) / SUM(t.total_denom_spent) 
                    ELSE 0 
                END as roi
            FROM eth_db.trades t
            JOIN eth_db.blocks b ON t.entry_block = b.block_number
            WHERE b.block_timestamp >= :start_timestamp
            GROUP BY time_bucket
            ORDER BY time_bucket ASC
        """)
        
        with self.Session() as session:
            result = session.execute(query, {"start_timestamp": start_timestamp})
            df = pd.DataFrame(result.fetchall(), columns=['time_bucket', 'total_profit', 'total_volume', 'roi'])
            
            # Convert time_bucket to datetime if it's not already
            if not pd.api.types.is_datetime64_any_dtype(df['time_bucket']):
                df['time_bucket'] = pd.to_datetime(df['time_bucket'])
            
            return {
                'timestamps': df['time_bucket'].dt.strftime('%Y-%m-%d %H:%M:%S').tolist(),
                'total_profit': df['total_profit'].fillna(0).tolist(),
                'total_volume': df['total_volume'].fillna(0).tolist(),
                'roi': df['roi'].fillna(0).tolist()
            }
    
    def _get_start_timestamp(self, timeframe: str) -> Optional[int]:
        """Helper method to calculate start timestamp based on timeframe."""
        now = datetime.now()
        if timeframe == 'day':
            return int((now - timedelta(days=1)).timestamp())
        elif timeframe == 'week':
            return int((now - timedelta(days=7)).timestamp())
        elif timeframe == 'month':
            return int((now - timedelta(days=30)).timestamp())
        elif timeframe == 'year':
            return int((now - timedelta(days=365)).timestamp())
        else:  # 'all' or any other value
            return 0  # Beginning of time
    
    def get_token_data(self, filters=None, limit=1000, include_trades=True, timeframe='all'):
        """
        Get token data for time-based analysis.
        
        This function retrieves token data and related trade metrics for analyzing token
        performance over time. It combines basic token information with aggregated trade
        statistics to provide insights into token creation, trading patterns, and lifecycle.
        
        Args:
            filters: Dict of filters to apply (e.g., {'is_scam': True})
            limit: Maximum number of tokens to return
            include_trades: Whether to include trade metrics 
            timeframe: Time period to consider ('day', 'week', 'month', 'year', 'all')
            
        Returns:
            pandas.DataFrame: Token data with trading metrics
        """
        start_timestamp = self._get_start_timestamp(timeframe)
        
        # Base query for token data
        base_query = """
        SELECT 
            t.contract_address,
            t.is_scam,
            t.scam_label,
            a.address as creator_address,
            tx.block_number as creation_block,
            b.block_timestamp as creation_timestamp
        FROM 
            eth_db.tokens t
        LEFT JOIN 
            eth_db.addresses a ON t.creator_address_id = a.address_id
        LEFT JOIN 
            eth_db.transactions tx ON t.creation_tx = tx.tx_hash
        LEFT JOIN 
            eth_db.blocks b ON tx.block_number = b.block_number
        WHERE 
            (:start_timestamp = 0 OR b.block_timestamp >= :start_timestamp)
        """
        
        # Apply filters if provided
        filter_conditions = []
        filter_params = {"start_timestamp": start_timestamp, "limit": limit}
        
        if filters:
            for key, value in filters.items():
                if key in ['is_scam', 'scam_label']:
                    filter_conditions.append(f"t.{key} = :{key}")
                    filter_params[key] = value
        
        if filter_conditions:
            base_query += " AND " + " AND ".join(filter_conditions)
        
        # Include trade metrics if requested
        if include_trades:
            query = f"""
            WITH token_base AS (
                {base_query}
            ),
            trade_metrics AS (
                SELECT 
                    tr.token_address,
                    COUNT(DISTINCT tr.id) AS total_trades,
                    SUM(tr.num_buys) AS total_buys,
                    SUM(tr.num_sells) AS total_sells,
                    SUM(tr.realized_profit) AS total_realized_profit,
                    SUM(tr.total_denom_spent) AS total_volume,
                    MAX(tr.latest_block) AS last_traded_block
                FROM 
                    eth_db.trades tr
                GROUP BY 
                    tr.token_address
            )
            SELECT 
                tb.*,
                COALESCE(tm.total_trades, 0) AS total_trades,
                COALESCE(tm.total_buys, 0) AS total_buys,
                COALESCE(tm.total_sells, 0) AS total_sells,
                COALESCE(tm.total_realized_profit, 0) AS total_realized_profit,
                COALESCE(tm.total_volume, 0) AS total_volume,
                tm.last_traded_block,
                CASE WHEN tm.total_buys > 0 THEN tm.total_sells / tm.total_buys ELSE 0 END AS sell_buy_ratio
            FROM 
                token_base tb
            LEFT JOIN 
                trade_metrics tm ON tb.contract_address = tm.token_address
            ORDER BY 
                tb.creation_timestamp DESC
            LIMIT :limit
            """
        else:
            query = f"{base_query} ORDER BY b.block_timestamp DESC LIMIT :limit"
        
        # Execute query
        with self.engine.connect() as conn:
            result = conn.execute(text(query), filter_params)
            columns = result.keys()
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            # Convert timestamp to datetime
            if 'creation_timestamp' in df.columns:
                df['creation_timestamp'] = pd.to_datetime(df['creation_timestamp'], unit='s')
            
            return df
    
    def get_token_lifecycle_metrics(self, token_address, interval='day'):
        """
        Get time series data for a specific token's lifecycle metrics.
        
        This function retrieves time-based metrics for a specific token, showing
        how its trading activity, volume, and other metrics changed over time.
        
        Args:
            token_address: The token contract address
            interval: Time interval for aggregation ('hour', 'day', 'week', 'month')
            
        Returns:
            pandas.DataFrame: Time series data for the token's lifecycle
        """
        # Define interval in PostgreSQL date_trunc format
        interval_map = {
            'hour': 'hour',
            'day': 'day',
            'week': 'week',
            'month': 'month'
        }
        pg_interval = interval_map.get(interval, 'day')
        
        query = text(f"""
            SELECT 
                date_trunc('{pg_interval}', to_timestamp(b.block_timestamp)) as time_bucket,
                COUNT(DISTINCT t.id) as trade_count,
                SUM(t.num_buys) as buy_count,
                SUM(t.num_sells) as sell_count,
                SUM(t.total_denom_spent) as volume,
                SUM(t.realized_profit) as profit
            FROM 
                eth_db.trades t
            JOIN 
                eth_db.blocks b ON t.entry_block = b.block_number
            WHERE 
                t.token_address = :token_address
            GROUP BY 
                time_bucket
            ORDER BY 
                time_bucket ASC
        """)
        
        with self.Session() as session:
            result = session.execute(query, {"token_address": token_address})
            df = pd.DataFrame(result.fetchall(), columns=[
                'timestamp', 'trade_count', 'buy_count', 'sell_count', 'volume', 'profit'
            ])
            
            # Convert time_bucket to datetime if needed
            if not pd.api.types.is_datetime64_any_dtype(df['timestamp']):
                df['timestamp'] = pd.to_datetime(df['timestamp'])
                
            # Calculate derived metrics
            if not df.empty:
                df['sell_buy_ratio'] = df['sell_count'] / df['buy_count'].replace(0, float('nan'))
                df['profit_volume_ratio'] = df['profit'] / df['volume'].replace(0, float('nan'))
                # Forward fill NaN values
                df = df.fillna(method='ffill')
                
            return df
    