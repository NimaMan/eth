"""
Helper functions for managing pool data in the minimal schema
"""

from typing import Dict, List, Optional
from sqlalchemy.orm import Session
from eth_data.database.db import Token, Trade


def update_token_pools(session: Session, token_address: str, pool_data: Dict):
    """
    Update the pools JSON field for a token
    
    Example pool_data:
    {
        "address": "0xabc123...",
        "type": "V2",
        "denom": "WETH",
        "denom_address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        "is_scam": False,
        "reserves": {
            "token": 1000000,
            "denom": 5.0
        }
    }
    """
    token = session.query(Token).filter_by(contract_address=token_address).first()
    if not token:
        return
    
    # Initialize pools if needed
    if not token.pools:
        token.pools = []
    
    # Check if pool already exists
    pool_exists = False
    for i, pool in enumerate(token.pools):
        if pool['address'] == pool_data['address']:
            # Update existing pool
            token.pools[i] = pool_data
            pool_exists = True
            break
    
    if not pool_exists:
        # Add new pool
        token.pools.append(pool_data)
    
    # Update counts
    token.num_pools = len(token.pools)
    token.num_healthy_pools = sum(1 for p in token.pools if not p.get('is_scam', False))
    token.num_scam_pools = sum(1 for p in token.pools if p.get('is_scam', False))
    
    # Update token scam status based on pools
    update_token_scam_status(token)
    
    # Mark the JSON field as modified (SQLAlchemy requirement)
    from sqlalchemy.orm.attributes import flag_modified
    flag_modified(token, "pools")


def update_token_scam_status(token: Token):
    """
    Update token-level scam status based on pool states
    """
    if not token.pools:
        return
    
    healthy_pools = [p for p in token.pools if not p.get('is_scam', False)]
    scam_pools = [p for p in token.pools if p.get('is_scam', False)]
    
    # Token is scam if:
    # 1. Has only one pool and it's a scam
    # 2. Has multiple pools and ALL are scams
    if len(token.pools) == 1 and len(scam_pools) == 1:
        token.is_scam = True
        token.scam_label = "Single pool drained"
    elif len(token.pools) > 1 and len(scam_pools) == len(token.pools):
        token.is_scam = True
        token.scam_label = "All pools drained"
    else:
        # Some pools are still healthy
        token.is_scam = False
        if scam_pools:
            token.scam_label = f"{len(scam_pools)} of {len(token.pools)} pools scam"
        else:
            token.scam_label = None


def mark_pool_as_scam(session: Session, token_address: str, pool_address: str, reason: str):
    """
    Mark a specific pool as scam
    """
    token = session.query(Token).filter_by(contract_address=token_address).first()
    if not token or not token.pools:
        return
    
    for pool in token.pools:
        if pool['address'] == pool_address:
            pool['is_scam'] = True
            pool['scam_reason'] = reason
            break
    
    # Update counts and token status
    token.num_healthy_pools = sum(1 for p in token.pools if not p.get('is_scam', False))
    token.num_scam_pools = sum(1 for p in token.pools if p.get('is_scam', False))
    update_token_scam_status(token)
    
    # Mark as modified
    from sqlalchemy.orm.attributes import flag_modified
    flag_modified(token, "pools")


def get_healthy_pools(token: Token) -> List[Dict]:
    """
    Get all healthy (non-scam) pools for a token
    """
    if not token.pools:
        return []
    return [p for p in token.pools if not p.get('is_scam', False)]


def get_pool_by_address(token: Token, pool_address: str) -> Optional[Dict]:
    """
    Get a specific pool by address
    """
    if not token.pools:
        return None
    
    for pool in token.pools:
        if pool['address'] == pool_address:
            return pool
    return None


def record_trade_with_pool(
    session: Session,
    trade: Trade,
    pool_address: str,
    pool_type: str = None
):
    """
    Update a trade record with pool information
    """
    trade.pool_address = pool_address
    
    if pool_type:
        trade.pool_type = pool_type
    
    # If this trade used multiple pools (e.g., split across pools)
    # you can track them in pools_used
    if not trade.pools_used:
        trade.pools_used = []
    
    if pool_address not in trade.pools_used:
        trade.pools_used.append({
            "address": pool_address,
            "type": pool_type
        })
        
        # Mark as modified
        from sqlalchemy.orm.attributes import flag_modified
        flag_modified(trade, "pools_used")

    