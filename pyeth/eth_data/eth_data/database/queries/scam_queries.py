"""
    - They have participated in at least a specified minimum number of trades.
    - Their ratio of scam-related trades to total trades is above a specified threshold.
    - They have recent activity on the chain (i.e. their latest trade occurred after a given block).
"""

from sqlalchemy import text
from typing import List, Any


def get_scam_addresses(min_trades: int, scam_ratio_threshold: float,  limit: int = 100) -> List[Any]:
    """
    Retrieve addresses that are frequently involved in scam token trades.

    Parameters:
        min_trades (int): The minimum number of trades the address must have participated in.
        scam_ratio_threshold (float): The minimum ratio (scam trades / total trades) required.
        limit (int): Maximum number of addresses to return.

    Returns:
        A list of rows (each row is typically a dictionary-like object) containing:
            - address
            - total_trades: Total number of trades by this address.
            - scam_trades: Number of trades that involved scam tokens.
            - scam_ratio: Calculated as scam_trades / total_trades.
            - latest_trade_block: The highest (most recent) entry_block seen for that address.
    """
    query = text("""
        SELECT a.address,
               COUNT(tr.id) AS total_trades,
               SUM(CASE WHEN t.is_scam = TRUE THEN 1 ELSE 0 END) AS scam_trades,
               (SUM(CASE WHEN t.is_scam = TRUE THEN 1 ELSE 0 END) / COUNT(tr.id))::float AS scam_ratio,
               MAX(tr.entry_block) AS latest_trade_block
        FROM eth_db.trades tr
        JOIN eth_db.tokens t ON tr.token_address = t.contract_address
        JOIN eth_db.addresses a ON tr.address = a.address
        GROUP BY a.address
        HAVING COUNT(tr.id) >= :min_trades 
           AND (SUM(CASE WHEN t.is_scam = TRUE THEN 1 ELSE 0 END) / COUNT(tr.id))::float > :scam_ratio_threshold
        ORDER BY scam_ratio DESC
        LIMIT :limit
    """)
    
    return query


def get_scam_tokens_query(limit: int = 100) -> List[Any]:
    """
    Retrieve tokens that are flagged as scams.

    Parameters:
        session: A SQLAlchemy session object.
        limit (int): Maximum number of tokens to return.

    Returns:
        A list of rows containing token metadata for tokens where is_scam is TRUE.
    """
    query = text("""
        SELECT *
        FROM eth_db.tokens
        WHERE is_scam = TRUE
        LIMIT :limit
    """)
    
    return query


def get_scam_label_distribution_query():
    """Get scam label distribution for pie chart"""
    query = text("""
        SELECT 
            scam_label as name,
            COUNT(DISTINCT contract_address) as value
        FROM eth_db.tokens 
        GROUP BY scam_label
        ORDER BY value DESC
    """)
    
    return query


def get_address_scam_trade_count_query():
    """Get address scam statistics from addresses table"""
    query = text("""
        SELECT 
            address,
            total_erc20_trades as total_trades,
            scam_ratio,
            total_profit,
            total_volume
        FROM eth_db.addresses
        ORDER BY total_erc20_trades DESC
    """)
    return query


def get_mimic_octopus_addresses_query() -> text:
    """Get addresses labeled as Mimic Octopus (deceptive volume creators)"""
    return text("""
        SELECT address, scam_ratio 
        FROM eth_db.addresses
        WHERE cluster_label = 'Mimic Octopus'
    """)