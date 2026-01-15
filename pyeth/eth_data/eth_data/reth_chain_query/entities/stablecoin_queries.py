"""
Stablecoin Query Functions using PyReth

This module provides functions to query stablecoin data directly from the blockchain
using pyreth's ChainQuery (Rust-based implementation) for high performance.

Key functions:
- get_stablecoin_total_supply: Get total supply of a specific stablecoin
- get_stablecoin_balance: Get stablecoin balance for a specific holder
- get_all_stablecoins_total_supply: Get total supply for all stablecoins
- get_holder_all_stablecoin_balances: Get all stablecoin balances for a holder
"""

from typing import Dict, Optional
from decimal import Decimal
from .base_queries import BaseEntityQuery, TimePeriod

from eth_data.chain_utils.common_addresses import (
    ERC20_TOKEN_DECIMALS,
    STABLECOINS_ADDRESS_BY_NAME,
    STABLECOIN_UNIT_BY_NAME,
)


class StablecoinQueries(BaseEntityQuery):
    """
    Stablecoin-specific queries for supply, balances, and market analysis.
    
    All calculations are performed in Rust for performance.
    Python only formats and presents the data.
    """
    
    def get_supply_changes_between_blocks(self, from_block: int, to_block: int) -> Dict:
        """
        Get stablecoin supply changes between two blocks.
        
        Uses PyReth's calculate_stablecoin_supply_changes_between_blocks.
        """
        return self.query.calculate_stablecoin_supply_changes_between_blocks(
            from_block, to_block
        )
    
    def get_market_overview(self, hours_back: int = 24) -> Dict:
        """
        Get complete stablecoin market overview for dashboard.
        
        Returns:
            Dict with total market cap, supply changes, top movers, etc.
        """
        start_block, end_block = self.convert_time_to_blocks(hours_back)
        
        # Get supply changes
        supply_changes = self.get_supply_changes_between_blocks(start_block, end_block)
        
        # Calculate market metrics
        total_mcap = 0
        top_movers = []
        
        for token, data in supply_changes.get('token_changes', {}).items():
            total_mcap += data.get('new_supply', 0)
            if abs(data.get('percent_change', 0)) > 0.01:  # More than 0.01% change
                top_movers.append({
                    'symbol': token,
                    'change': data['net_change'],
                    'percent': data['percent_change']
                })
        
        # Sort by absolute change
        top_movers.sort(key=lambda x: abs(x['percent']), reverse=True)
        
        return {
            'total_market_cap': total_mcap,
            'total_minted': supply_changes.get('total_minted', 0),
            'total_burned': supply_changes.get('total_burned', 0),
            'net_supply_change': supply_changes.get('total_net_change', 0),
            'top_movers': top_movers[:5],
            'block_range': {'from': start_block, 'to': end_block}
        }


# Legacy function support - to be deprecated
_queries = None

def _get_chain_query():
    """Get or create StablecoinQueries instance for legacy functions"""
    global _queries
    if _queries is None:
        _queries = StablecoinQueries()
    return _queries.query


def get_stablecoin_total_supply(
    token_name: str, 
    block_number: Optional[int] = None
) -> Dict[str, any]:
    """
    Get total supply of a specific stablecoin.
    
    Args:
        token_name: Name of the stablecoin (e.g., 'USDC', 'USDT', 'DAI')
        block_number: Optional block number to query at (default: latest)
    
    Returns:
        Dict containing:
            - name: Token name
            - address: Token contract address
            - total_supply_raw: Raw total supply (in smallest unit)
            - total_supply: Human-readable total supply
            - decimals: Token decimals
            - unit: What the token represents (e.g., 'US Dollar')
            - block_number: Block number queried
    """
    if token_name not in STABLECOINS_ADDRESS_BY_NAME:
        raise ValueError(f"Unknown stablecoin: {token_name}")
    
    token_address = STABLECOINS_ADDRESS_BY_NAME[token_name]
    decimals = ERC20_TOKEN_DECIMALS.get(token_name, 18)
    unit = STABLECOIN_UNIT_BY_NAME.get(token_name, "Unknown")
    
    chain_query = _get_chain_query()
    
    # Get total supply from blockchain
    total_supply_raw = chain_query.get_token_total_supply(token_address, block_number)
    
    # Convert to human-readable format
    total_supply_decimal = Decimal(total_supply_raw) / Decimal(10 ** decimals)
    
    # Get actual block number if not specified
    if block_number is None:
        actual_block = chain_query.get_latest_block()
    else:
        actual_block = block_number
    
    return {
        "name": token_name,
        "address": token_address,
        "total_supply_raw": total_supply_raw,
        "total_supply": float(total_supply_decimal),
        "total_supply_formatted": f"{total_supply_decimal:,.2f}",
        "decimals": decimals,
        "unit": unit,
        "block_number": actual_block
    }


def get_stablecoin_balance(
    token_name: str,
    holder_address: str,
    block_number: Optional[int] = None
) -> Dict[str, any]:
    """
    Get stablecoin balance for a specific holder.
    
    Args:
        token_name: Name of the stablecoin (e.g., 'USDC', 'USDT', 'DAI')
        holder_address: Address to check balance for
        block_number: Optional block number to query at (default: latest)
    
    Returns:
        Dict containing:
            - name: Token name
            - token_address: Token contract address
            - holder_address: Holder address
            - balance_raw: Raw balance (in smallest unit)
            - balance: Human-readable balance
            - decimals: Token decimals
            - unit: What the token represents
            - block_number: Block number queried
    """
    if token_name not in STABLECOINS_ADDRESS_BY_NAME:
        raise ValueError(f"Unknown stablecoin: {token_name}")
    
    token_address = STABLECOINS_ADDRESS_BY_NAME[token_name]
    decimals = ERC20_TOKEN_DECIMALS.get(token_name, 18)
    unit = STABLECOIN_UNIT_BY_NAME.get(token_name, "Unknown")
    
    chain_query = _get_chain_query()
    
    # Get balance from blockchain
    balance_raw = chain_query.get_token_balance(token_address, holder_address, block_number)
    
    # Convert to human-readable format
    balance_decimal = Decimal(balance_raw) / Decimal(10 ** decimals)
    
    # Get actual block number if not specified
    if block_number is None:
        actual_block = chain_query.get_latest_block()
    else:
        actual_block = block_number
    
    return {
        "name": token_name,
        "token_address": token_address,
        "holder_address": holder_address,
        "balance_raw": balance_raw,
        "balance": float(balance_decimal),
        "balance_formatted": f"{balance_decimal:,.2f}",
        "decimals": decimals,
        "unit": unit,
        "block_number": actual_block
    }


def get_all_stablecoins_total_supply(
    block_number: Optional[int] = None,
    include_failed: bool = False
) -> Dict[str, Dict]:
    """
    Get total supply for all known stablecoins.
    
    Args:
        block_number: Optional block number to query at (default: latest)
        include_failed: Whether to include stablecoins that failed to query
    
    Returns:
        Dict mapping token name to supply data (same format as get_stablecoin_total_supply)
    """
    results = {}
    
    for token_name in STABLECOINS_ADDRESS_BY_NAME:
        try:
            supply_data = get_stablecoin_total_supply(token_name, block_number)
            results[token_name] = supply_data
        except Exception as e:
            if include_failed:
                results[token_name] = {
                    "name": token_name,
                    "address": STABLECOINS_ADDRESS_BY_NAME[token_name],
                    "error": str(e)
                }
            # Skip failed queries if not including them
    
    return results


def get_holder_all_stablecoin_balances(
    holder_address: str,
    block_number: Optional[int] = None,
    only_non_zero: bool = True
) -> Dict[str, Dict]:
    """
    Get all stablecoin balances for a specific holder.
    
    Args:
        holder_address: Address to check balances for
        block_number: Optional block number to query at (default: latest)
        only_non_zero: Whether to only return non-zero balances
    
    Returns:
        Dict mapping token name to balance data (same format as get_stablecoin_balance)
    """
    results = {}
    
    for token_name in STABLECOINS_ADDRESS_BY_NAME:
        try:
            balance_data = get_stablecoin_balance(token_name, holder_address, block_number)
            
            # Skip zero balances if requested
            if only_non_zero and balance_data["balance"] == 0:
                continue
                
            results[token_name] = balance_data
        except Exception as e:
            # Skip failed queries
            continue
    
    return results


def compare_top_stablecoins_supply(block_number: Optional[int] = None) -> Dict[str, Dict]:
    """
    Get total supply for top stablecoins (USDC, USDT, DAI) for quick comparison.
    
    Args:
        block_number: Optional block number to query at (default: latest)
    
    Returns:
        Dict with supply data for USDC, USDT, and DAI
    """
    top_stables = ["USDC", "USDT", "DAI"]
    results = {}
    
    for token_name in top_stables:
        try:
            results[token_name] = get_stablecoin_total_supply(token_name, block_number)
        except Exception as e:
            results[token_name] = {"error": str(e)}
    
    return results
