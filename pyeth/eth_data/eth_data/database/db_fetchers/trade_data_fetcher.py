"""
Trade Data Retriever Module

Objective:
---------
Provide methods for retrieving and analyzing trade-related data from the database,
including trade details, profit/loss metrics, and trade relationships.

Key Features:
-----------
1. Trade Lookup: Find specific trades by various criteria
2. PnL Analysis: Calculate profit and loss metrics for trades
3. Related Address Analysis: Discover network relationships from trade data
4. Performance Metrics: Calculate ROI, volume metrics across timeframes
5. Specialized Views: Find trades with highest buy volume, sell volume, bribes, or ratio
"""

from sqlalchemy import text, select, func
from eth_data.database.eth_db_conn import get_db_session_maker, get_db_engine
import pandas as pd
from datetime import datetime, timedelta
from typing import List, Dict, Optional, Union
import numpy as np
import math
from eth_data.database.schema.eth_db_data_models import Address
from sarigoz.utils.time_block_converter import TimeBlockConverter
from web3 import Web3


class TradeDataFetcher:
    
    def __init__(self, w3=None):
        """
        Initialize the TradeDataFetcher with database connections and utilities.
        
        Args:
            w3 (Web3, optional): Web3 instance to use for block-time conversion
        """
        self.Session = get_db_session_maker(db='eth_db')
        self.engine = get_db_engine(db='eth_db')
        
        # Initialize web3 if provided or try to create one
        if w3:
            self.w3 = w3
        else:
            try:
                # Try to connect to local Ethereum node
                self.w3 = Web3(Web3.HTTPProvider("http://localhost:8545"))
                if not self.w3.is_connected():
                    print("Warning: Could not connect to Ethereum node, block-time conversion will use estimation")
                    self.w3 = None
            except Exception:
                self.w3 = None
        
        # Initialize TimeBlockConverter with web3 instance
        self.block_converter = TimeBlockConverter(w3=self.w3)
    
    def _replace_nan_inf_with_none(self, obj):
        """Recursively replace NaN and Infinity in dicts/lists with None for JSON serialization."""
        if isinstance(obj, dict):
            return {k: self._replace_nan_inf_with_none(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [self._replace_nan_inf_with_none(elem) for elem in obj]
        elif isinstance(obj, float) and (math.isnan(obj) or math.isinf(obj)):
            return None
        elif isinstance(obj, np.floating) and (np.isnan(obj) or np.isinf(obj)):
            return None
        return obj
    
    def _sanitize_dataframe(self, df: pd.DataFrame) -> pd.DataFrame:
        """Replace NaN and Inf values in DataFrame with None for JSON serialization."""
        # Replace NaN and Inf with None in numeric columns
        numeric_cols = df.select_dtypes(include=[np.number]).columns
        for col in numeric_cols:
            df[col] = df[col].replace([np.nan, np.inf, -np.inf], None)
        return df
    
    def _convert_to_python_int(self, value: Union[int, np.int64]) -> int:
        """Convert numpy.int64 to Python int."""
        if isinstance(value, np.int64):
            return int(value)
        return value
    
    def _get_address_id(self, session, address: str) -> Optional[int]:
        """Helper method to get address_id from address string."""
        stmt = select(Address.address_id).where(Address.address == address)
        result = session.execute(stmt).scalar_one_or_none()
        return result
    
    def _get_block_filter(self, timeframe: Optional[str] = None, 
                        start_date: Optional[datetime] = None, 
                        end_date: Optional[datetime] = None) -> tuple[str, dict]:
        """
        Helper method to generate block range filter SQL and parameters.
        
        Args:
            timeframe: Predefined time period ('24h', '7d', '30d', 'all')
            start_date: Custom start date (overrides timeframe if provided)
            end_date: Custom end date (overrides timeframe if provided)
            
        Returns:
            tuple[str, dict]: SQL WHERE clause fragment and parameters dict
        """
        block_filter = ""
        params = {}
        
        # If explicit dates are provided, they take precedence over timeframe
        if start_date or end_date:
            start_block, end_block = self.block_converter.time_range_to_block_range(start_date, end_date)
        elif timeframe and timeframe != 'all':
            # Get the latest block information for accurate 24h data
            if timeframe == '24h':
                # Make sure we have the most up-to-date block information
                self.block_converter = TimeBlockConverter(w3=self.w3)
            
            start_block, end_block = self.block_converter.timeframe_to_block_range(timeframe)
        else:
            # No time filtering
            return block_filter, params
            
        # Apply block range filter if we have bounds
        if start_block:
            block_filter += " AND t.entry_block >= :start_block"
            params['start_block'] = start_block
        if end_block:
            block_filter += " AND t.entry_block <= :end_block"
            params['end_block'] = end_block
            
        return block_filter, params
    
    def get_trade_by_id(self, trade_id):
        """
        Get detailed information about a specific trade.
        
        Args:
            trade_id: ID of the trade
            
        Returns:
            dict: Trade details
        """
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.id,
                    t.address_id,
                    t.token_address,
                    t.currency,
                    t.entry_block,
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
                    t.tx_fee,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address_id = a.address_id
                WHERE 
                    t.id = :trade_id
            """)

            result = session.execute(
                query, 
                {"trade_id": self._convert_to_python_int(trade_id)}
            )
            
            row = result.fetchone()
            if row:
                # Get address string for address_id
                address_id = row[1]
                if address_id:
                    stmt = select(Address.address).where(Address.address_id == address_id)
                    address = session.execute(stmt).scalar_one_or_none()
                else:
                    address = None
                    
                return {
                    "id": row[0],
                    "address": address,
                    "token_address": row[2],
                    "currency": row[3],
                    "entry_block": row[4],
                    "total_denom_spent": row[5],
                    "total_denom_received": row[6],
                    "denom_received_spent_ratio": row[7],
                    "bribe_amount": row[8],
                    "realized_profit": row[9],
                    "unrealized_profit": row[10],
                    "num_buys": row[11],
                    "num_sells": row[12],
                    "token_holdings_ratio": row[13],
                    "token_sell_buy_ratio": row[14],
                    "agg_denom_balance": row[15],
                    "agg_token_balance": row[16],
                    "tx_fee": row[17],
                    "is_contract_address": row[18]
                }
            return None
    
    def get_most_profitable_trades(
            self, 
            limit: int = 25,
            min_profit: Optional[float] = None,
            timeframe: str = 'all',
            start_date: Optional[datetime] = None,
            end_date: Optional[datetime] = None) -> pd.DataFrame:
        """
        Get the most profitable trades, with support for time-based filtering.

        Args:
            limit (int): Max number of trades to return
            min_profit (Optional[float]): Minimum realized profit threshold
            timeframe (str): Predefined time period ('24h', '7d', '30d', 'all')
            start_date (Optional[datetime]): Custom start date (overrides timeframe if provided)
            end_date (Optional[datetime]): Custom end date (overrides timeframe if provided)
            
        Returns:
            pd.DataFrame: DataFrame containing the most profitable trades matching the criteria
        """
        engine = self.engine # Use the instance engine
        
        # Get block-based time filter
        block_filter, time_params = self._get_block_filter(timeframe, start_date, end_date)

        # Initialize params dict with the time parameters
        params = {**time_params, 'limit': self._convert_to_python_int(limit)}

        # Add profit filter
        min_profit_filter = ""
        if min_profit is not None:
            min_profit_filter = " AND t.realized_profit >= :min_profit"
            params['min_profit'] = min_profit
        else:
            # Ensure we are actually looking for PROFITABLE trades if no min_profit is set
            min_profit_filter = " AND t.realized_profit > 0 "
            
        # Optimized query with block-based time filtering
        query = text(f"""
            SELECT 
                t.id,
                a.address, -- Select address string directly
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
                t.tx_fee,
                a.is_contract
            FROM eth_db.trades t
            LEFT JOIN eth_db.addresses a ON t.address_id = a.address_id
            WHERE t.entry_block >= 21000000 {min_profit_filter} {block_filter}
            ORDER BY t.realized_profit DESC
            LIMIT :limit
        """)

        columns = [
            'id', 'address', 'token_address', 'currency', 'entry_block', 'latest_block',
            'total_denom_spent', 'total_denom_received', 'denom_received_spent_ratio',
            'bribe_amount', 'realized_profit', 'unrealized_profit', 'num_buys',
            'num_sells', 'token_holdings_ratio', 'token_sell_buy_ratio',
            'agg_denom_balance', 'agg_token_balance', 'tx_fee', 'is_contract' 
        ]

        with engine.connect() as connection:
            result = connection.execute(query, params)
            df = pd.DataFrame(result.fetchall(), columns=columns)

        # Convert numeric columns safely
        numeric_cols = ['total_denom_spent', 'total_denom_received',
                      'denom_received_spent_ratio', 'bribe_amount', 'realized_profit',
                      'unrealized_profit', 'token_holdings_ratio', 'token_sell_buy_ratio',
                      'agg_denom_balance', 'agg_token_balance', 'tx_fee']
        for col in numeric_cols:
            if col in df.columns:
                df[col] = pd.to_numeric(df[col], errors='coerce')

        # Convert boolean column
        if 'is_contract' in df.columns:
            df['is_contract'] = df['is_contract'].fillna(False).astype(bool)

        # Sanitize NaN and Inf values for JSON serialization
        df = self._sanitize_dataframe(df)

        return df
    
    def get_biggest_loss_trades(self, limit: int = 25, max_loss: Optional[float] = None,
                              timeframe: str = 'all',
                              start_date: Optional[datetime] = None, 
                              end_date: Optional[datetime] = None) -> pd.DataFrame:
        """
        Get trades with the biggest losses, with support for time-based filtering.

        Args:
            limit (int): Max number of trades to return
            max_loss (Optional[float]): Maximum loss threshold (negative value)
            timeframe (str): Predefined time period ('24h', '7d', '30d', 'all')
            start_date (Optional[datetime]): Custom start date (overrides timeframe if provided)
            end_date (Optional[datetime]): Custom end date (overrides timeframe if provided)

        Returns:
            pd.DataFrame: DataFrame containing the biggest loss trades matching the criteria
        """
        engine = self.engine
        
        # Get block-based time filter
        block_filter, time_params = self._get_block_filter(timeframe, start_date, end_date)
        
        # Initialize params dict with the time parameters
        params = {**time_params, 'limit': self._convert_to_python_int(limit)}
        
        # Add loss filter
        loss_filter = " AND t.realized_profit < 0"
        if max_loss is not None:
            loss_filter += " AND t.realized_profit >= :max_loss"
            params['max_loss'] = max_loss

        # Optimized query with block-based time filtering
        query = text(f"""
            SELECT 
                t.id,
                a.address, -- Select address string directly
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
                t.tx_fee,
                a.is_contract
            FROM eth_db.trades t
            LEFT JOIN eth_db.addresses a ON t.address_id = a.address_id
            WHERE t.entry_block >= 21000000 {loss_filter} {block_filter}
            ORDER BY t.realized_profit ASC
            LIMIT :limit
        """)

        columns = [
            'id', 'address', 'token_address', 'currency', 'entry_block', 'latest_block',
            'total_denom_spent', 'total_denom_received', 'denom_received_spent_ratio',
            'bribe_amount', 'realized_profit', 'unrealized_profit', 'num_buys',
            'num_sells', 'token_holdings_ratio', 'token_sell_buy_ratio',
            'agg_denom_balance', 'agg_token_balance', 'tx_fee', 'is_contract' 
        ]

        with engine.connect() as connection:
            result = connection.execute(query, params)
            df = pd.DataFrame(result.fetchall(), columns=columns)

        # Convert numeric columns safely
        numeric_cols = ['total_denom_spent', 'total_denom_received',
                      'denom_received_spent_ratio', 'bribe_amount', 'realized_profit',
                      'unrealized_profit', 'token_holdings_ratio', 'token_sell_buy_ratio',
                      'agg_denom_balance', 'agg_token_balance', 'tx_fee']
        for col in numeric_cols:
            if col in df.columns:
                df[col] = pd.to_numeric(df[col], errors='coerce')

        # Convert boolean column
        if 'is_contract' in df.columns:
            df['is_contract'] = df['is_contract'].fillna(False).astype(bool)

        # Sanitize NaN and Inf values for JSON serialization
        df = self._sanitize_dataframe(df)

        return df
    
    def get_recent_trades(self, limit: int = 25, offset: int = 0,
                         timeframe: str = 'all',
                         start_date: Optional[datetime] = None, 
                         end_date: Optional[datetime] = None) -> pd.DataFrame:
        """
        Get recent trades with support for time-based filtering.

        Args:
            limit (int): Max number of trades to return
            offset (int): Number of trades to skip
            timeframe (str): Predefined time period ('24h', '7d', '30d', 'all')
            start_date (Optional[datetime]): Custom start date (overrides timeframe if provided)
            end_date (Optional[datetime]): Custom end date (overrides timeframe if provided)

        Returns:
            pd.DataFrame: DataFrame containing recent trades matching the criteria
        """
        engine = self.engine
        
        # Get block-based time filter
        block_filter, time_params = self._get_block_filter(timeframe, start_date, end_date)
        
        # Initialize params dict with the time parameters
        params = {**time_params, 'limit': self._convert_to_python_int(limit), 'offset': self._convert_to_python_int(offset)}

        # Optimized query with block-based time filtering
        query = text(f"""
            SELECT
                t.id,
                a.address, -- Select address string directly
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
                t.tx_fee,
                a.is_contract 
            FROM eth_db.trades t
            LEFT JOIN eth_db.addresses a ON t.address_id = a.address_id
            WHERE t.entry_block >= 21000000 {block_filter}
            ORDER BY t.entry_block DESC
            LIMIT :limit
            OFFSET :offset
        """)

        columns = [
            'id', 'address', 'token_address', 'currency', 'entry_block', 'latest_block',
            'total_denom_spent', 'total_denom_received', 'denom_received_spent_ratio',
            'bribe_amount', 'realized_profit', 'unrealized_profit', 'num_buys',
            'num_sells', 'token_holdings_ratio', 'token_sell_buy_ratio',
            'agg_denom_balance', 'agg_token_balance', 'tx_fee', 'is_contract' 
        ]

        with engine.connect() as connection:
            result = connection.execute(query, params)
            df = pd.DataFrame(result.fetchall(), columns=columns)
                            
        # Convert numeric columns safely
        numeric_cols = ['total_denom_spent', 'total_denom_received',
                      'denom_received_spent_ratio', 'bribe_amount', 'realized_profit',
                      'unrealized_profit', 'token_holdings_ratio', 'token_sell_buy_ratio',
                      'agg_denom_balance', 'agg_token_balance', 'tx_fee']
        for col in numeric_cols:
            if col in df.columns:
                df[col] = pd.to_numeric(df[col], errors='coerce')

        # Convert boolean column
        if 'is_contract' in df.columns:
            df['is_contract'] = df['is_contract'].fillna(False).astype(bool)

        # Sanitize NaN and Inf values for JSON serialization
        df = self._sanitize_dataframe(df)

        return df
    
    def get_trader_rankings(self, limit: int = 100, timeframe: str = 'all',
                           start_date: Optional[datetime] = None, 
                           end_date: Optional[datetime] = None) -> pd.DataFrame:
        """
        Get trader rankings aggregated by address with comprehensive scoring.

        Args:
            limit (int): Max number of traders to return
            timeframe (str): Predefined time period ('24h', '7d', '30d', 'all')
            start_date (Optional[datetime]): Custom start date (overrides timeframe if provided)
            end_date (Optional[datetime]): Custom end date (overrides timeframe if provided)

        Returns:
            pd.DataFrame: DataFrame containing trader rankings with scores and metrics
        """
        engine = self.engine
        
        # Get block-based time filter
        block_filter, time_params = self._get_block_filter(timeframe, start_date, end_date)
        
        # Initialize params dict with the time parameters
        params = {**time_params, 'limit': self._convert_to_python_int(limit)}

        # Comprehensive trader aggregation query
        query = text(f"""
            WITH trader_stats AS (
                SELECT 
                    a.address,
                    a.address_id,
                    a.is_contract,
                    -- Core Performance Metrics
                    COUNT(t.id) as total_trades,
                    SUM(t.total_denom_spent) as total_volume_spent,
                    SUM(t.total_denom_received) as total_volume_received,
                    SUM(t.realized_profit) as total_realized_profit,
                    SUM(t.unrealized_profit) as total_unrealized_profit,
                    SUM(t.bribe_amount) as total_bribes,
                    SUM(t.tx_fee) as total_fees,
                    SUM(t.num_buys) as total_buys,
                    SUM(t.num_sells) as total_sells,
                    
                    -- Performance Ratios
                    AVG(t.denom_received_spent_ratio) as avg_return_ratio,
                    COUNT(CASE WHEN t.realized_profit > 0 THEN 1 END) as winning_trades,
                    COUNT(CASE WHEN t.realized_profit < 0 THEN 1 END) as losing_trades,
                    
                    -- Risk Metrics
                    MAX(t.realized_profit) as best_trade,
                    MIN(t.realized_profit) as worst_trade,
                    STDDEV(t.realized_profit) as profit_volatility,
                    
                    -- Activity Metrics
                    MIN(t.entry_block) as first_trade_block,
                    MAX(t.latest_block) as last_trade_block,
                    COUNT(DISTINCT t.token_address) as unique_tokens_traded
                    
                FROM eth_db.trades t
                LEFT JOIN eth_db.addresses a ON t.address_id = a.address_id
                WHERE t.entry_block >= 21000000 {block_filter}
                    AND a.is_contract = false  -- Only human traders
                GROUP BY a.address, a.address_id, a.is_contract
                HAVING COUNT(t.id) >= 3  -- Minimum 3 trades for ranking
            ),
            trader_scores AS (
                SELECT *,
                    -- Win Rate (0-100)
                    CASE 
                        WHEN total_trades > 0 THEN (winning_trades::float / total_trades::float * 100)
                        ELSE 0 
                    END as win_rate,
                    
                    -- Profit Efficiency (profit per trade)
                    CASE 
                        WHEN total_trades > 0 THEN (total_realized_profit / total_trades)
                        ELSE 0 
                    END as profit_per_trade,
                    
                    -- Volume Efficiency (profit per volume)
                    CASE 
                        WHEN total_volume_spent > 0 THEN (total_realized_profit / total_volume_spent * 100)
                        ELSE 0 
                    END as roi_percentage,
                    
                    -- Activity Score (trades per time period)
                    CASE 
                        WHEN (last_trade_block - first_trade_block) > 0 
                        THEN (total_trades::float / (last_trade_block - first_trade_block) * 100000)
                        ELSE 0 
                    END as activity_score
                    
                FROM trader_stats
            )
            SELECT 
                address,
                total_trades,
                total_realized_profit,
                total_volume_spent,
                total_volume_received,
                win_rate,
                profit_per_trade,
                roi_percentage,
                activity_score,
                total_buys,
                total_sells,
                unique_tokens_traded,
                best_trade,
                worst_trade,
                profit_volatility,
                total_bribes,
                total_fees,
                
                -- Composite Score (0-1000)
                LEAST(1000, 
                    -- Profit Component (40%)
                    (CASE 
                        WHEN total_realized_profit > 0 THEN LEAST(400, LOG(total_realized_profit + 1) * 40)
                        ELSE 0 
                    END) +
                    
                    -- Win Rate Component (30%)
                    (win_rate * 3) +
                    
                    -- Volume Component (20%)
                    (CASE 
                        WHEN total_volume_spent > 0 THEN LEAST(200, LOG(total_volume_spent + 1) * 15)
                        ELSE 0 
                    END) +
                    
                    -- Activity Component (10%)
                    (LEAST(100, activity_score * 10))
                ) as composite_score
                
            FROM trader_scores
            WHERE total_realized_profit IS NOT NULL
            ORDER BY composite_score DESC, total_realized_profit DESC
            LIMIT :limit
        """)

        columns = [
            'address', 'total_trades', 'total_realized_profit', 'total_volume_spent',
            'total_volume_received', 'win_rate', 'profit_per_trade', 'roi_percentage',
            'activity_score', 'total_buys', 'total_sells', 'unique_tokens_traded',
            'best_trade', 'worst_trade', 'profit_volatility', 'total_bribes',
            'total_fees', 'composite_score'
        ]

        with engine.connect() as connection:
            result = connection.execute(query, params)
            df = pd.DataFrame(result.fetchall(), columns=columns)

        # Convert numeric columns safely
        numeric_cols = [
            'total_trades', 'total_realized_profit', 'total_volume_spent', 'total_volume_received',
            'win_rate', 'profit_per_trade', 'roi_percentage', 'activity_score',
            'total_buys', 'total_sells', 'unique_tokens_traded', 'best_trade',
            'worst_trade', 'profit_volatility', 'total_bribes', 'total_fees', 'composite_score'
        ]
        for col in numeric_cols:
            if col in df.columns:
                df[col] = pd.to_numeric(df[col], errors='coerce')

        # Add ranking position
        df['rank'] = range(1, len(df) + 1)
        
        # Sanitize NaN and Inf values for JSON serialization
        df = self._sanitize_dataframe(df)

        return df
    
    def get_biggest_buys(self, limit=25, min_spent=None, include_scams=True):
        """
        Get trades with the highest buy volume based on total_denom_spent (filtered for ETH currency).
        
        Args:
            limit: Maximum number of trades to return
            min_spent: Minimum amount spent threshold
            include_scams (bool): Whether to include trades involving tokens marked as scams (default True).
            
        Returns:
            pandas.DataFrame: Highest buy volume ETH trades
        """
        scam_filter = "" if include_scams else "AND tk.is_scam = FALSE"

        with self.Session() as session:
            query = text(f"""
                SELECT 
                    t.*,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address_id = a.address_id
                JOIN 
                    eth_db.tokens tk ON t.token_address = tk.contract_address
                WHERE 
                    t.currency = 'ETH'
                    AND t.total_denom_spent > 0
                    AND t.num_buys > 0
                    {scam_filter}
                    {" AND t.total_denom_spent >= :min_spent" if min_spent is not None else ""}
                ORDER BY 
                    t.total_denom_spent DESC
                LIMIT :limit
            """)

            params = {"limit": self._convert_to_python_int(limit)}
            if min_spent is not None:
                params["min_spent"] = self._convert_to_python_int(min_spent)
                
            result = session.execute(query, params)
            
            # Get column names from result description
            column_names = [col[0] for col in result.cursor.description]
            
            df = pd.DataFrame(result.fetchall(), columns=column_names)
            
            # Get address strings for address_ids
            address_ids = df['address_id'].unique()
            if len(address_ids) > 0:
                address_map = self._get_address_map(session, address_ids)
                df['address'] = df['address_id'].map(address_map)
                            
            return df
    
    def get_biggest_sells(self, limit=25, min_received=None, include_scams=True):
        """
        Get trades with the highest sell volume based on total_denom_received (filtered for ETH currency).
        
        Args:
            limit: Maximum number of trades to return
            min_received: Minimum amount received threshold
            include_scams (bool): Whether to include trades involving tokens marked as scams (default True).
            
        Returns:
            pandas.DataFrame: Highest sell volume ETH trades
        """
        scam_filter = "" if include_scams else "AND tk.is_scam = FALSE"

        with self.Session() as session:
            query = text(f"""
                SELECT 
                    t.*,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address_id = a.address_id
                JOIN 
                    eth_db.tokens tk ON t.token_address = tk.contract_address
                WHERE 
                    t.currency = 'ETH'
                    AND t.total_denom_received > 0
                    AND t.num_sells > 0
                    {scam_filter}
                    {" AND t.total_denom_received >= :min_received" if min_received is not None else ""}
                ORDER BY 
                    t.total_denom_received DESC
                LIMIT :limit
            """)

            params = {"limit": self._convert_to_python_int(limit)}
            if min_received is not None:
                params["min_received"] = self._convert_to_python_int(min_received)
                
            result = session.execute(query, params)
            
            # Get column names from result description
            column_names = [col[0] for col in result.cursor.description]
            
            df = pd.DataFrame(result.fetchall(), columns=column_names)
            
            # Get address strings for address_ids
            address_ids = df['address_id'].unique()
            if len(address_ids) > 0:
                address_map = self._get_address_map(session, address_ids)
                df['address'] = df['address_id'].map(address_map)
                            
            return df
    
    def get_biggest_bribes(self, limit=25, min_bribe=None, include_scams=True):
        """
        Get trades with the highest bribe amounts (filtered for ETH currency).
        
        Args:
            limit: Maximum number of trades to return
            min_bribe: Minimum bribe amount threshold
            include_scams (bool): Whether to include trades involving tokens marked as scams (default True).
            
        Returns:
            pandas.DataFrame: Highest bribe ETH trades
        """
        scam_filter = "" if include_scams else "AND tk.is_scam = FALSE"

        with self.Session() as session:
            query = text(f"""
                SELECT 
                    t.*,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address_id = a.address_id
                JOIN 
                    eth_db.tokens tk ON t.token_address = tk.contract_address
                WHERE 
                    t.currency = 'ETH'
                    AND t.bribe_amount > 0
                    {scam_filter}
                    {" AND t.bribe_amount >= :min_bribe" if min_bribe is not None else ""}
                ORDER BY 
                    t.bribe_amount DESC
                LIMIT :limit
            """)

            params = {"limit": self._convert_to_python_int(limit)}
            if min_bribe is not None:
                params["min_bribe"] = self._convert_to_python_int(min_bribe)
                
            result = session.execute(query, params)
            
            # Get column names from result description
            column_names = [col[0] for col in result.cursor.description]
            
            df = pd.DataFrame(result.fetchall(), columns=column_names)
            
            # Get address strings for address_ids
            address_ids = df['address_id'].unique()
            if len(address_ids) > 0:
                address_map = self._get_address_map(session, address_ids)
                df['address'] = df['address_id'].map(address_map)
                            
            return df
    
    def get_best_received_spent_ratio(self, limit=25, min_ratio=None, include_scams=True):
        """
        Get trades with the highest received-to-spent ratio (filtered for ETH currency).
        
        Args:
            limit: Maximum number of trades to return
            min_ratio: Minimum ratio threshold
            include_scams (bool): Whether to include trades involving tokens marked as scams (default True).
            
        Returns:
            pandas.DataFrame: Highest received-to-spent ratio ETH trades
        """
        scam_filter = "" if include_scams else "AND tk.is_scam = FALSE"

        with self.Session() as session:
            query = text(f"""
                SELECT 
                    t.*,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address_id = a.address_id
                JOIN 
                    eth_db.tokens tk ON t.token_address = tk.contract_address
                WHERE 
                    t.currency = 'ETH'
                    AND t.total_denom_spent > 0
                    AND t.denom_received_spent_ratio > 0
                    {scam_filter}
                    {" AND t.denom_received_spent_ratio >= :min_ratio" if min_ratio is not None else ""}
                ORDER BY 
                    t.denom_received_spent_ratio DESC
                LIMIT :limit
            """)

            params = {"limit": self._convert_to_python_int(limit)}
            if min_ratio is not None:
                params["min_ratio"] = self._convert_to_python_int(min_ratio)
                
            result = session.execute(query, params)
            
            # Get column names from result description
            column_names = [col[0] for col in result.cursor.description]
            
            df = pd.DataFrame(result.fetchall(), columns=column_names)
                            
            # Get address strings for address_ids
            address_ids = df['address_id'].unique()
            if len(address_ids) > 0:
                address_map = self._get_address_map(session, address_ids)
                df['address'] = df['address_id'].map(address_map)
            
            return df
    
    def get_related_addresses_by_trade(self, trade_id):
        """
        Get addresses related to a specific trade.
        
        Args:
            trade_id: ID of the trade
            
        Returns:
            pandas.DataFrame: Related addresses and relationship details
        """
        # Define expected columns
        columns = ['address_id', 'related_address_id', 'denom_flow', 'trade_id',
                  'is_address_contract', 'is_related_address_contract',
                  'address_profit', 'related_address_profit']
        
        with self.Session() as session:
            query = text("""
                SELECT 
                    ra.address_id,
                    ra.related_address_id,
                    ra.denom_flow,
                    ra.trade_id,
                    a1.is_contract as is_address_contract,
                    a2.is_contract as is_related_address_contract,
                    a1.total_realized_profit as address_profit,
                    a2.total_realized_profit as related_address_profit
                FROM 
                    eth_db.related_addresses ra
                LEFT JOIN
                    eth_db.addresses a1 ON ra.address_id = a1.address_id
                LEFT JOIN
                    eth_db.addresses a2 ON ra.related_address_id = a2.address_id
                WHERE 
                    ra.trade_id = :trade_id
                ORDER BY 
                    ra.denom_flow DESC
            """)

            result = session.execute(
                query, 
                {"trade_id": self._convert_to_python_int(trade_id)}
            )
            
            # Create DataFrame with expected columns
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            # Get address strings for address_ids
            address_ids = set(df['address_id'].unique()) | set(df['related_address_id'].unique())
            if len(address_ids) > 0:
                address_map = self._get_address_map(session, list(address_ids))
                df['address'] = df['address_id'].map(address_map)
                df['related_address'] = df['related_address_id'].map(address_map)
            else:
                # Add empty address columns if no data
                df['address'] = pd.Series(dtype=str)
                df['related_address'] = pd.Series(dtype=str)
            
            # Ensure proper data types
            df['denom_flow'] = df['denom_flow'].astype(float)
            df['address_profit'] = df['address_profit'].astype(float)
            df['related_address_profit'] = df['related_address_profit'].astype(float)
            df['is_address_contract'] = df['is_address_contract'].astype(bool)
            df['is_related_address_contract'] = df['is_related_address_contract'].astype(bool)
                            
            return df
    
    def get_trades_by_timeframe(self, timeframe: str = 'all', limit: int = 100) -> List[Dict]:
        """
        Get trades within a specific time period.
        
        Args:
            timeframe: Time period to analyze ('24h', '7d', '30d', 'all')
            limit: Maximum number of trades to return
            
        Returns:
            List of trade dictionaries
        """
        # Get block-based time filter using the TimeBlockConverter
        block_filter, time_params = self._get_block_filter(timeframe)
        
        # Initialize params dict with the time parameters
        params = {**time_params, 'limit': self._convert_to_python_int(limit)}
        
        query = text(f"""
            SELECT 
                t.address_id,
                t.token_address,
                t.realized_profit,
                t.total_denom_spent,
                t.total_denom_received,
                t.num_buys,
                t.num_sells,
                t.entry_block
            FROM eth_db.trades t
            WHERE t.entry_block >= 21000000 {block_filter}
            ORDER BY t.entry_block DESC
            LIMIT :limit
        """)
        
        with self.Session() as session:
            result = session.execute(query, params)
            trades = result.fetchall()
            
            # Get address strings for address_ids
            address_ids = [trade[0] for trade in trades if trade[0]]
            if len(address_ids) > 0:
                address_map = self._get_address_map(session, address_ids)
            else:
                address_map = {}
            
            # Process results
            trade_list = []
            for trade in trades:
                # Convert block to timestamp if available
                try:
                    entry_block = trade[7]
                    timestamp = self.block_converter.block_to_timestamp(entry_block)
                    timestamp_str = datetime.fromtimestamp(timestamp).strftime('%Y-%m-%d %H:%M:%S')
                except:
                    timestamp_str = "N/A"
                
                trade_list.append({
                    'address': address_map.get(trade[0]),
                    'token_address': trade[1],
                    'profit': float(trade[2]) if trade[2] is not None else 0.0,
                    'volume': float(trade[3]) if trade[3] is not None else 0.0,
                    'received': float(trade[4]) if trade[4] is not None else 0.0,
                    'num_buys': trade[5] if trade[5] is not None else 0,
                    'num_sells': trade[6] if trade[6] is not None else 0,
                    'timestamp': timestamp_str
                })
                
            return trade_list
    
    def _get_address_map(self, session, address_ids: List[Union[int, np.int64]]) -> Dict[int, str]:
        """Helper method to get address strings for a list of address IDs."""
        if not address_ids:
            return {}
            
        # Convert numpy int64 to Python int if needed
        address_ids = [self._convert_to_python_int(id_) for id_ in address_ids]
        
        # Query for all addresses in one go
        stmt = select(Address.address_id, Address.address).where(Address.address_id.in_(address_ids))
        results = session.execute(stmt).fetchall()
        
        # Create mapping
        return {row[0]: row[1] for row in results}
        
    def get_last_24h_token_pnl(self, token_address: str, limit: int = 25) -> pd.DataFrame:
        """
        Get the latest 24h PnL data for a specific token, optimized for frontend display.
        
        This method uses the TimeBlockConverter to accurately determine the block range
        for the last 24 hours, ensuring the frontend shows the most recent data.
        
        Args:
            token_address (str): Token contract address
            limit (int): Maximum number of trades to return
            
        Returns:
            pd.DataFrame: DataFrame containing the PnL data for the last 24 hours
        """
        # Ensure we have the latest block information
        self.block_converter = TimeBlockConverter(w3=self.w3)
        
        # Get block range for last 24 hours
        start_block, end_block = self.block_converter.timeframe_to_block_range('24h')
        
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.id,
                    a.address,
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
                    t.tx_fee,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address_id = a.address_id
                WHERE 
                    t.token_address = :token_address
                    AND t.entry_block >= :start_block
                    AND t.latest_block <= :end_block
                ORDER BY 
                    t.realized_profit DESC
                LIMIT :limit
            """)

            result = session.execute(
                query, 
                {
                    "token_address": token_address,
                    "start_block": start_block,
                    "end_block": end_block,
                    "limit": limit
                }
            )
            
            columns = ['id', 'address', 'token_address', 'currency', 'entry_block', 
                       'latest_block', 'total_denom_spent', 'total_denom_received', 
                       'denom_received_spent_ratio', 'bribe_amount',
                       'realized_profit', 'unrealized_profit', 
                       'num_buys', 'num_sells', 'token_holdings_ratio',
                       'token_sell_buy_ratio', 'agg_denom_balance', 'agg_token_balance',
                       'tx_fee', 'is_contract']
            
            df = pd.DataFrame(result.fetchall(), columns=columns)
            return df 