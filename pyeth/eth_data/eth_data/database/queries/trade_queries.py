"""
    - How does the PNL distribution look for a given token?
    - What are the PNL details for a specific address?
"""

from sqlalchemy import text


GET_PNL_DISTRIBUTION_BY_TOKEN = """
SELECT tr.address, tr.realized_profit, tr.unrealized_profit, tr.total_profit
FROM pnl.trades tr
WHERE tr.token_address = :token_address
ORDER BY tr.total_profit DESC
"""

GET_ADDRESS_PNL = """
SELECT address, total_profit, total_volume, trade_frequency, first_seen, last_seen
FROM pnl.addresses
WHERE address = :address
"""


def get_trades_for_address_query(address: str):
    """
    Get all trades for a specific address
    """
    query = text("""SELECT * FROM pnl.trades WHERE address = :address""")
    return query, {'address': address}


def get_high_unrealized_profit_query(min_profit: float = 1e3):  # 1000 ETH
    """
    Find addresses with suspiciously high unrealized profits in PnL analysis
    Returns details about trades and tokens to help investigate numerical issues
    
    Args:
        min_profit (float): Minimum unrealized profit threshold in ETH (default: 1000)
    Returns:
        Tuple of (SQLAlchemy query, parameters dict)
    """
    query = text("""
        SELECT 
            a.address,
            t.token_address,
            t.unrealized_profit,
            t.agg_token_balance,
            t.total_denom_spent,
            t.total_denom_received,
            t.entry_block
        FROM pnl.trades t
        JOIN pnl.addresses a ON t.address = a.address
        WHERE t.unrealized_profit >= :min_profit
        ORDER BY t.unrealized_profit DESC
    """)
    return query, {'min_profit': min_profit}


def get_top_traders_query(offset: int = 0, limit: int = 100):
    """
    Get traders sorted by total_realized_profit, paginated with existing columns
    """
    query = text("""
        SELECT 
            address,
            total_erc20_trades,
            total_realized_profit,
            total_volume,
            total_denom_balance,
            mean_received_spent_ratio,
            median_received_spent_ratio,
            avg_bribe_amount,
            total_bribe_amount,
            median_entry_block,
            num_related_addresses,
            agg_profit_with_related,
            cluster_label,
            scam_ratio,
            trade_frequency,
            first_seen,
            last_seen
        FROM pnl.addresses
        WHERE total_realized_profit IS NOT NULL
        ORDER BY total_realized_profit DESC
        LIMIT :limit
        OFFSET :offset
    """)
    
    return query, {"limit": limit, "offset": offset}