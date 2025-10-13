"""
Token Pool Launch Stats Fetcher
--------------------------------

Fetches pool launch statistics from the database, treating each token-pool pair
as an independent launch event. Uses block timestamps for all time-based analysis.

Key Concepts:
- Each pool launch is an independent data point (no aggregation across pools)
- Pool trading_enabled_block joined with blocks.block_timestamp for launch time
- Scam detection tracked at pool level with timestamps
- All methods return native Python dicts/lists (no pandas)
"""

from typing import List, Dict, Optional, Any
from datetime import datetime, timedelta
from sqlalchemy import text

from eth_data.database.eth_db_conn import get_db_session_maker
from eth_data.utils.logger import get_logger


class TokenPoolLaunchStatsFetcher:
    """
    Fetches pool launch statistics treating each token-pool pair as a launch event.
    Uses block timestamps for all time-based analysis.
    """
    
    def __init__(self):
        """Initialize the fetcher with database connection and logger."""
        self.Session = get_db_session_maker(db='eth_db')
        self.logger = get_logger(__name__)
    
    def get_pool_launches_last_n_days(
        self, 
        days: int = 1, 
        limit: int = 100, 
        offset: int = 0
    ) -> List[Dict]:
        """
        Get pools launched in the last N days.
        
        Args:
            days: Number of days to look back
            limit: Maximum number of pools to return
            offset: Number of pools to skip for pagination
            
        Returns:
            List of pool launch dictionaries
        """
        cutoff_time = datetime.now() - timedelta(days=days)
        cutoff_timestamp = int(cutoff_time.timestamp())
        
        try:
            with self.Session() as session:
                query = text("""
                    SELECT 
                        p.pool_address,
                        p.pool_id,
                        p.pool_type,
                        p.token_address,
                        p.pair_token_address,
                        p.fee_tier,
                        p.trading_enabled_block as launch_block,
                        p.trading_enabled_tx as launch_tx,
                        b_launch.block_timestamp as launch_timestamp,
                        p.is_scam,
                        p.scam_label,
                        p.scam_block,
                        p.scam_tx_hash,
                        b_scam.block_timestamp as scam_timestamp,
                        t.contract_address as token_address,
                        t.creator_address_id,
                        a_creator.address as creator_address
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    LEFT JOIN 
                        eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                    LEFT JOIN 
                        eth_db.tokens t ON p.token_address = t.contract_address
                    LEFT JOIN 
                        eth_db.addresses a_creator ON t.creator_address_id = a_creator.address_id
                    WHERE 
                        b_launch.block_timestamp >= :cutoff_timestamp
                        AND p.trading_enabled = true
                    ORDER BY 
                        b_launch.block_timestamp DESC
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
                
                pools = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    
                    pool_dict = {
                        "pool_address": row_map.get('pool_address'),
                        "pool_id": row_map.get('pool_id'),
                        "pool_type": row_map.get('pool_type'),
                        "token_address": row_map.get('token_address'),
                        "pair_token": row_map.get('pair_token_address'),
                        "fee_tier": row_map.get('fee_tier'),
                        "launch_block": row_map.get('launch_block'),
                        "launch_tx": row_map.get('launch_tx'),
                        "launch_timestamp": row_map.get('launch_timestamp'),
                        "launch_datetime": datetime.fromtimestamp(row_map['launch_timestamp']) if row_map.get('launch_timestamp') else None,
                        "is_scam": row_map.get('is_scam'),
                        "scam_label": row_map.get('scam_label'),
                        "scam_block": row_map.get('scam_block'),
                        "scam_tx": row_map.get('scam_tx_hash'),
                        "scam_timestamp": row_map.get('scam_timestamp'),
                        "creator_address": row_map.get('creator_address'),
                        "hours_since_launch": None,
                        "time_to_scam_hours": None
                    }
                    
                    # Calculate time metrics
                    if pool_dict['launch_timestamp']:
                        launch_dt = datetime.fromtimestamp(pool_dict['launch_timestamp'])
                        pool_dict['hours_since_launch'] = (datetime.now() - launch_dt).total_seconds() / 3600
                        
                        if pool_dict['scam_timestamp']:
                            scam_dt = datetime.fromtimestamp(pool_dict['scam_timestamp'])
                            pool_dict['time_to_scam_hours'] = (scam_dt - launch_dt).total_seconds() / 3600
                    
                    pools.append(pool_dict)
                
                return pools
                
        except Exception as e:
            self.logger.error(f"Error fetching pool launches for last {days} days: {e}")
            return []
    
    def get_pool_launches_by_timestamp_range(
        self,
        start_timestamp: Optional[int] = None,
        end_timestamp: Optional[int] = None,
        limit: int = 100,
        offset: int = 0
    ) -> List[Dict]:
        """
        Get pools launched within a timestamp range.
        
        Args:
            start_timestamp: Start timestamp (inclusive)
            end_timestamp: End timestamp (inclusive)
            limit: Maximum number of pools to return
            offset: Number of pools to skip
            
        Returns:
            List of pool launch dictionaries
        """
        try:
            with self.Session() as session:
                # Build WHERE conditions
                where_conditions = ["p.trading_enabled = true"]
                params = {"limit": limit, "offset": offset}
                
                if start_timestamp:
                    where_conditions.append("b_launch.block_timestamp >= :start_timestamp")
                    params["start_timestamp"] = start_timestamp
                    
                if end_timestamp:
                    where_conditions.append("b_launch.block_timestamp <= :end_timestamp")
                    params["end_timestamp"] = end_timestamp
                
                where_clause = " AND ".join(where_conditions)
                
                query = text(f"""
                    SELECT 
                        p.pool_address,
                        p.pool_id,
                        p.pool_type,
                        p.token_address,
                        p.pair_token_address,
                        p.fee_tier,
                        p.trading_enabled_block as launch_block,
                        p.trading_enabled_tx as launch_tx,
                        b_launch.block_timestamp as launch_timestamp,
                        p.is_scam,
                        p.scam_label,
                        p.scam_block,
                        p.scam_tx_hash,
                        b_scam.block_timestamp as scam_timestamp
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    LEFT JOIN 
                        eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                    WHERE 
                        {where_clause}
                    ORDER BY 
                        b_launch.block_timestamp DESC
                    LIMIT :limit OFFSET :offset
                """)
                
                result = session.execute(query, params)
                
                pools = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    
                    pool_dict = {
                        "pool_address": row_map.get('pool_address'),
                        "pool_id": row_map.get('pool_id'),
                        "pool_type": row_map.get('pool_type'),
                        "token_address": row_map.get('token_address'),
                        "pair_token": row_map.get('pair_token_address'),
                        "fee_tier": row_map.get('fee_tier'),
                        "launch_block": row_map.get('launch_block'),
                        "launch_tx": row_map.get('launch_tx'),
                        "launch_timestamp": row_map.get('launch_timestamp'),
                        "is_scam": row_map.get('is_scam'),
                        "scam_label": row_map.get('scam_label'),
                        "scam_block": row_map.get('scam_block'),
                        "scam_tx": row_map.get('scam_tx_hash'),
                        "scam_timestamp": row_map.get('scam_timestamp')
                    }
                    
                    # Calculate time to scam if applicable
                    if pool_dict['launch_timestamp'] and pool_dict['scam_timestamp']:
                        pool_dict['time_to_scam_hours'] = (
                            pool_dict['scam_timestamp'] - pool_dict['launch_timestamp']
                        ) / 3600
                    else:
                        pool_dict['time_to_scam_hours'] = None
                    
                    pools.append(pool_dict)
                
                return pools
                
        except Exception as e:
            self.logger.error(f"Error fetching pool launches by timestamp range: {e}")
            return []
    
    def get_pool_launch_stats_by_type(
        self,
        start_timestamp: Optional[int] = None,
        end_timestamp: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Get launch statistics grouped by pool type (V2/V3/V4).
        
        Args:
            start_timestamp: Start timestamp for filtering
            end_timestamp: End timestamp for filtering
            
        Returns:
            Dictionary with stats per pool type
        """
        try:
            with self.Session() as session:
                # Build WHERE conditions
                where_conditions = ["p.trading_enabled = true"]
                params = {}
                
                if start_timestamp:
                    where_conditions.append("b_launch.block_timestamp >= :start_timestamp")
                    params["start_timestamp"] = start_timestamp
                    
                if end_timestamp:
                    where_conditions.append("b_launch.block_timestamp <= :end_timestamp")
                    params["end_timestamp"] = end_timestamp
                
                where_clause = " AND ".join(where_conditions)
                
                query = text(f"""
                    SELECT 
                        p.pool_type,
                        COUNT(*) as total_launches,
                        COUNT(CASE WHEN p.is_scam = true THEN 1 END) as scam_count,
                        AVG(CASE 
                            WHEN p.scam_block IS NOT NULL 
                            THEN (b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0
                            ELSE NULL 
                        END) as avg_time_to_scam_hours,
                        MIN(b_launch.block_timestamp) as earliest_launch,
                        MAX(b_launch.block_timestamp) as latest_launch
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    LEFT JOIN 
                        eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                    WHERE 
                        {where_clause}
                    GROUP BY 
                        p.pool_type
                    ORDER BY 
                        total_launches DESC
                """)
                
                result = session.execute(query, params)
                
                stats_by_type = {}
                total_launches = 0
                total_scams = 0
                
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    pool_type = row_map['pool_type']
                    launches = row_map['total_launches'] or 0
                    scams = row_map['scam_count'] or 0
                    
                    stats_by_type[pool_type] = {
                        "total_launches": launches,
                        "scam_count": scams,
                        "scam_rate": (scams / launches * 100) if launches > 0 else 0,
                        "avg_time_to_scam_hours": row_map.get('avg_time_to_scam_hours'),
                        "earliest_launch": datetime.fromtimestamp(row_map['earliest_launch']) if row_map.get('earliest_launch') else None,
                        "latest_launch": datetime.fromtimestamp(row_map['latest_launch']) if row_map.get('latest_launch') else None
                    }
                    
                    total_launches += launches
                    total_scams += scams
                
                return {
                    "by_type": stats_by_type,
                    "total_launches": total_launches,
                    "total_scams": total_scams,
                    "overall_scam_rate": (total_scams / total_launches * 100) if total_launches > 0 else 0
                }
                
        except Exception as e:
            self.logger.error(f"Error fetching pool launch stats by type: {e}")
            return {"by_type": {}, "total_launches": 0, "total_scams": 0, "overall_scam_rate": 0}
    
    def get_pool_launch_velocity(
        self,
        days_back: int = 7,
        group_by_hours: int = 24
    ) -> List[Dict]:
        """
        Get pool launch velocity over time periods.
        
        Args:
            days_back: Number of days to analyze
            group_by_hours: Group launches by this many hours (default 24 = daily)
            
        Returns:
            List of time period dictionaries with launch counts
        """
        try:
            cutoff_time = datetime.now() - timedelta(days=days_back)
            cutoff_timestamp = int(cutoff_time.timestamp())
            
            with self.Session() as session:
                # Calculate time buckets in seconds
                bucket_seconds = group_by_hours * 3600
                
                query = text("""
                    SELECT 
                        (b_launch.block_timestamp / :bucket_seconds) * :bucket_seconds as time_bucket,
                        COUNT(*) as launches,
                        COUNT(CASE WHEN p.is_scam = true THEN 1 END) as scams,
                        COUNT(DISTINCT p.token_address) as unique_tokens
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    WHERE 
                        b_launch.block_timestamp >= :cutoff_timestamp
                        AND p.trading_enabled = true
                    GROUP BY 
                        time_bucket
                    ORDER BY 
                        time_bucket ASC
                """)
                
                result = session.execute(
                    query,
                    {
                        "bucket_seconds": bucket_seconds,
                        "cutoff_timestamp": cutoff_timestamp
                    }
                )
                
                velocity = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    
                    velocity.append({
                        "timestamp": int(row_map['time_bucket']),
                        "datetime": datetime.fromtimestamp(row_map['time_bucket']),
                        "launches": row_map['launches'] or 0,
                        "scams": row_map['scams'] or 0,
                        "unique_tokens": row_map['unique_tokens'] or 0,
                        "scam_rate": (row_map['scams'] / row_map['launches'] * 100) if row_map['launches'] > 0 else 0,
                        "launches_per_hour": row_map['launches'] / group_by_hours if group_by_hours > 0 else 0
                    })
                
                return velocity
                
        except Exception as e:
            self.logger.error(f"Error fetching pool launch velocity: {e}")
            return []
    
    def get_scam_pools_last_n_days(
        self,
        days: int = 1,
        limit: int = 100
    ) -> List[Dict]:
        """
        Get scam pools detected in the last N days.
        
        Args:
            days: Number of days to look back
            limit: Maximum number of pools to return
            
        Returns:
            List of scam pool dictionaries
        """
        cutoff_time = datetime.now() - timedelta(days=days)
        cutoff_timestamp = int(cutoff_time.timestamp())
        
        try:
            with self.Session() as session:
                query = text("""
                    SELECT 
                        p.pool_address,
                        p.pool_id,
                        p.pool_type,
                        p.token_address,
                        p.pair_token_address,
                        p.fee_tier,
                        p.trading_enabled_block as launch_block,
                        b_launch.block_timestamp as launch_timestamp,
                        p.scam_label,
                        p.scam_block,
                        p.scam_tx_hash,
                        b_scam.block_timestamp as scam_timestamp,
                        t.creator_address_id,
                        a_creator.address as creator_address
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    LEFT JOIN 
                        eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                    LEFT JOIN 
                        eth_db.tokens t ON p.token_address = t.contract_address
                    LEFT JOIN 
                        eth_db.addresses a_creator ON t.creator_address_id = a_creator.address_id
                    WHERE 
                        b_scam.block_timestamp >= :cutoff_timestamp
                        AND p.is_scam = true
                    ORDER BY 
                        b_scam.block_timestamp DESC
                    LIMIT :limit
                """)
                
                result = session.execute(
                    query,
                    {
                        "cutoff_timestamp": cutoff_timestamp,
                        "limit": limit
                    }
                )
                
                scam_pools = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    
                    pool_dict = {
                        "pool_address": row_map.get('pool_address'),
                        "pool_id": row_map.get('pool_id'),
                        "pool_type": row_map.get('pool_type'),
                        "token_address": row_map.get('token_address'),
                        "pair_token": row_map.get('pair_token_address'),
                        "fee_tier": row_map.get('fee_tier'),
                        "launch_block": row_map.get('launch_block'),
                        "launch_timestamp": row_map.get('launch_timestamp'),
                        "scam_label": row_map.get('scam_label'),
                        "scam_block": row_map.get('scam_block'),
                        "scam_tx": row_map.get('scam_tx_hash'),
                        "scam_timestamp": row_map.get('scam_timestamp'),
                        "creator_address": row_map.get('creator_address'),
                        "time_to_scam_hours": None
                    }
                    
                    # Calculate time to scam
                    if pool_dict['launch_timestamp'] and pool_dict['scam_timestamp']:
                        pool_dict['time_to_scam_hours'] = (
                            pool_dict['scam_timestamp'] - pool_dict['launch_timestamp']
                        ) / 3600
                    
                    scam_pools.append(pool_dict)
                
                return scam_pools
                
        except Exception as e:
            self.logger.error(f"Error fetching scam pools for last {days} days: {e}")
            return []
    
    def get_time_to_scam_stats(
        self,
        start_timestamp: Optional[int] = None,
        end_timestamp: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Get statistics on time between pool launch and scam detection.
        
        Args:
            start_timestamp: Start timestamp for filtering
            end_timestamp: End timestamp for filtering
            
        Returns:
            Dictionary with time-to-scam statistics
        """
        try:
            with self.Session() as session:
                # Build WHERE conditions
                where_conditions = [
                    "p.is_scam = true",
                    "p.scam_block IS NOT NULL",
                    "p.trading_enabled_block IS NOT NULL"
                ]
                params = {}
                
                if start_timestamp:
                    where_conditions.append("b_launch.block_timestamp >= :start_timestamp")
                    params["start_timestamp"] = start_timestamp
                    
                if end_timestamp:
                    where_conditions.append("b_launch.block_timestamp <= :end_timestamp")
                    params["end_timestamp"] = end_timestamp
                
                where_clause = " AND ".join(where_conditions)
                
                query = text(f"""
                    SELECT 
                        AVG((b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0) as avg_hours,
                        MIN((b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0) as min_hours,
                        MAX((b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0) as max_hours,
                        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY (b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0) as median_hours,
                        COUNT(*) as total_scams
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    LEFT JOIN 
                        eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                    WHERE 
                        {where_clause}
                """)
                
                result = session.execute(query, params)
                row = result.fetchone()
                
                if row:
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    
                    # Get distribution buckets
                    bucket_query = text(f"""
                        SELECT 
                            CASE 
                                WHEN hours < 1 THEN '< 1 hour'
                                WHEN hours < 6 THEN '1-6 hours'
                                WHEN hours < 24 THEN '6-24 hours'
                                WHEN hours < 72 THEN '1-3 days'
                                ELSE '> 3 days'
                            END as time_bucket,
                            COUNT(*) as count
                        FROM (
                            SELECT 
                                (b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0 as hours
                            FROM 
                                eth_db.pools p
                            LEFT JOIN 
                                eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                            LEFT JOIN 
                                eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                            WHERE {where_clause}
                        ) as time_diffs
                        GROUP BY time_bucket
                        ORDER BY 
                            CASE time_bucket
                                WHEN '< 1 hour' THEN 1
                                WHEN '1-6 hours' THEN 2
                                WHEN '6-24 hours' THEN 3
                                WHEN '1-3 days' THEN 4
                                ELSE 5
                            END
                    """)
                    
                    bucket_result = session.execute(bucket_query, params)
                    distribution = []
                    for bucket_row in bucket_result.fetchall():
                        bucket_map = bucket_row._mapping if hasattr(bucket_row, '_mapping') else dict(zip(bucket_result.keys(), bucket_row))
                        distribution.append({
                            "bucket": bucket_map['time_bucket'],
                            "count": bucket_map['count']
                        })
                    
                    return {
                        "avg_hours": row_map.get('avg_hours'),
                        "min_hours": row_map.get('min_hours'),
                        "max_hours": row_map.get('max_hours'),
                        "median_hours": row_map.get('median_hours'),
                        "total_scams": row_map.get('total_scams', 0),
                        "distribution": distribution
                    }
                
                return {
                    "avg_hours": None,
                    "min_hours": None,
                    "max_hours": None,
                    "median_hours": None,
                    "total_scams": 0,
                    "distribution": []
                }
                
        except Exception as e:
            self.logger.error(f"Error fetching time to scam stats: {e}")
            return {
                "avg_hours": None,
                "min_hours": None,
                "max_hours": None,
                "median_hours": None,
                "total_scams": 0,
                "distribution": []
            }
    
    def get_pool_launch_summary(
        self,
        start_timestamp: Optional[int] = None,
        end_timestamp: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Get comprehensive summary of pool launches.
        
        Args:
            start_timestamp: Start timestamp for filtering
            end_timestamp: End timestamp for filtering
            
        Returns:
            Dictionary with comprehensive launch statistics
        """
        try:
            with self.Session() as session:
                # Build WHERE conditions
                where_conditions = ["p.trading_enabled = true"]
                params = {}
                
                if start_timestamp:
                    where_conditions.append("b_launch.block_timestamp >= :start_timestamp")
                    params["start_timestamp"] = start_timestamp
                    
                if end_timestamp:
                    where_conditions.append("b_launch.block_timestamp <= :end_timestamp")
                    params["end_timestamp"] = end_timestamp
                
                where_clause = " AND ".join(where_conditions)
                
                query = text(f"""
                    SELECT 
                        COUNT(*) as total_pool_launches,
                        COUNT(DISTINCT p.token_address) as unique_tokens,
                        COUNT(CASE WHEN p.is_scam = true THEN 1 END) as scam_pools,
                        COUNT(CASE WHEN p.pool_type = 'V2' THEN 1 END) as v2_pools,
                        COUNT(CASE WHEN p.pool_type = 'V3' THEN 1 END) as v3_pools,
                        COUNT(CASE WHEN p.pool_type = 'V4' THEN 1 END) as v4_pools,
                        MIN(b_launch.block_timestamp) as earliest_launch,
                        MAX(b_launch.block_timestamp) as latest_launch,
                        AVG(CASE 
                            WHEN p.scam_block IS NOT NULL 
                            THEN (b_scam.block_timestamp - b_launch.block_timestamp) / 3600.0
                            ELSE NULL 
                        END) as avg_time_to_scam_hours
                    FROM 
                        eth_db.pools p
                    LEFT JOIN 
                        eth_db.blocks b_launch ON p.trading_enabled_block = b_launch.block_number
                    LEFT JOIN 
                        eth_db.blocks b_scam ON p.scam_block = b_scam.block_number
                    WHERE 
                        {where_clause}
                """)
                
                result = session.execute(query, params)
                row = result.fetchone()
                
                if row:
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    
                    total_launches = row_map.get('total_pool_launches', 0)
                    scam_pools = row_map.get('scam_pools', 0)
                    
                    # Calculate time range
                    time_range_hours = None
                    if row_map.get('earliest_launch') and row_map.get('latest_launch'):
                        time_range_hours = (row_map['latest_launch'] - row_map['earliest_launch']) / 3600
                    
                    return {
                        "total_pool_launches": total_launches,
                        "unique_tokens": row_map.get('unique_tokens', 0),
                        "scam_pools": scam_pools,
                        "scam_rate": (scam_pools / total_launches * 100) if total_launches > 0 else 0,
                        "launches_by_type": {
                            "V2": row_map.get('v2_pools', 0),
                            "V3": row_map.get('v3_pools', 0),
                            "V4": row_map.get('v4_pools', 0)
                        },
                        "avg_time_to_scam_hours": row_map.get('avg_time_to_scam_hours'),
                        "earliest_launch": datetime.fromtimestamp(row_map['earliest_launch']) if row_map.get('earliest_launch') else None,
                        "latest_launch": datetime.fromtimestamp(row_map['latest_launch']) if row_map.get('latest_launch') else None,
                        "time_range_hours": time_range_hours,
                        "launches_per_hour": total_launches / time_range_hours if time_range_hours and time_range_hours > 0 else 0
                    }
                
                return {
                    "total_pool_launches": 0,
                    "unique_tokens": 0,
                    "scam_pools": 0,
                    "scam_rate": 0,
                    "launches_by_type": {"V2": 0, "V3": 0, "V4": 0},
                    "avg_time_to_scam_hours": None,
                    "earliest_launch": None,
                    "latest_launch": None,
                    "time_range_hours": None,
                    "launches_per_hour": 0
                }
                
        except Exception as e:
            self.logger.error(f"Error fetching pool launch summary: {e}")
            return {
                "total_pool_launches": 0,
                "unique_tokens": 0,
                "scam_pools": 0,
                "scam_rate": 0,
                "launches_by_type": {"V2": 0, "V3": 0, "V4": 0},
                "avg_time_to_scam_hours": None,
                "earliest_launch": None,
                "latest_launch": None,
                "time_range_hours": None,
                "launches_per_hour": 0
            }