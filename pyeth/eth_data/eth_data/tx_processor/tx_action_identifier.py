from web3 import Web3
from typing import Dict, Any, List


class TransactionActionIdentifier:
    """
    Classifies transaction types by analyzing function signatures and logs
    
    Objective:
    ---------
    Determine the specific action(s) taken in a transaction by analyzing:
    1. Function signatures
    2. Event logs
    3. Event sequences and relationships
    
    This helps in accurate alert generation by understanding the context
    of token transfers and other events.
    """
    def identify_transaction_actions(self, 
                                    tx_type: str,
                                    parsed_logs: Dict[str, List[Any]]) -> List[str]:
        """
        Determine all actions in the transaction based on log analysis
        
        Returns a list of actions like:
        - "Token Swap"
        - "Add Liquidity"
        - "Remove Liquidity"
        - "Token Transfer"
        - "Token Receive"
        - "Trading Enable"
        - "Contract Creation"
        etc.
        """
        actions = []
        
        # Check for contract creation
        if tx_type == 'Contract Creation':
            actions.append("Contract Creation")
        
        if tx_type == 'Trading Enable':
            actions.append("Trading Enable")
        elif tx_type == 'Trading Disable':
            actions.append("Trading Disable")
        
        # Check for liquidity actions
        if self._is_add_liquidity_action(parsed_logs):
            actions.append("Add Liquidity")
        
        # Check for swaps
        if self._is_swap_action(parsed_logs):
            actions.append("Swap")

        # Check for ownership changes
        if parsed_logs.get('owner_events', []):
            actions.append("Ownership Change")
    
        return tuple(set(actions))  # Remove duplicates
    
    def _is_add_liquidity_action(self, parsed_logs: Dict[str, List[Any]]) -> bool:
        """Check for add liquidity pattern in logs for both Uniswap V2 and V3"""
        return self._is_add_liquidity_action_v2(parsed_logs) or self._is_add_liquidity_action_v3(parsed_logs)
    
    def _is_add_liquidity_action_v2(self, parsed_logs: Dict[str, List[Any]]) -> bool:
        """Check for add liquidity pattern in logs"""
        has_sync = len(parsed_logs.get('uniswap_v2_syncs', [])) > 0
        has_mint = len(parsed_logs.get('mints', [])) > 0
        return has_sync and has_mint

    def _is_add_liquidity_action_v3(self, parsed_logs: Dict[str, List[Any]]) -> bool:
        """
        Check for Uniswap V3 add liquidity pattern in logs
        
        Objective:
        ---------
        Identify Uniswap V3 liquidity addition by checking for the following pattern:
        1. Pool creation (optional - only for new pools)
        2. Pool initialization (optional - only for new pools)
        3. NFT position minting
        4. Token transfers to the pool
        5. WETH deposit (if ETH is used)
        
        The function handles both cases:
        - New pool creation + liquidity addition
        - Adding liquidity to existing pool
        """
        # Check for position minting (required for all liquidity additions)
        has_position = len(parsed_logs.get('uniswap_v3_positions', [])) > 0
        if not has_position:
            return False
            
        # Check for token transfers (required for all liquidity additions)
        has_transfers = len(parsed_logs.get('erc20_transfers', [])) >= 2  # Need at least 2 transfers (token0 and token1)
        if not has_transfers:
            return False
            
        # Check for pool creation and initialization (optional - only for new pools)
        has_pool_created = len(parsed_logs.get('uniswap_v3_pools', [])) > 0
        has_initialization = len(parsed_logs.get('uniswap_v3_initializations', [])) > 0
        
        # Case 1: New pool creation + liquidity
        if has_pool_created:
            return has_initialization and has_position and has_transfers
            
        # Case 2: Adding liquidity to existing pool
        return has_position and has_transfers
    
    def _is_swap_action(self, parsed_logs: Dict[str, List[Any]]) -> bool:
        """Check for swap pattern in logs"""
        has_swap = len(parsed_logs.get('uniswap_v2_swaps', [])) > 0
        return has_swap
    

    def _is_token_transfer_action(self, parsed_logs: Dict[str, List[Any]], transaction: Dict[str, Any]) -> bool:
        """
        Check for token transfer pattern in logs.
        
        A transaction is considered a pure token transfer if:
        1. Contains ERC20 transfer events
        2. Does NOT contain:
           - Swap events (DEX trading)
           - Pair events (Liquidity operations)
           - Contract creation events
           - Mint/Burn events (Supply changes)
           - Trading enable/disable events
        3. Is not a contract interaction (direct transfer)
        
        Returns:
        --------
        bool: True if the transaction is a pure token transfer
        """
        # Check for ERC20 transfers
        has_transfer = len(parsed_logs.get('erc20_transfers', [])) > 0
        if not has_transfer:
            return False
        
        # Check for other events that would indicate this isn't a pure transfer
        has_swap = len(parsed_logs.get('uniswap_v2_swaps', [])) > 0
        has_pair = len(parsed_logs.get('pair_events', [])) > 0
        has_mint = len(parsed_logs.get('mints', [])) > 0
        has_burn = len(parsed_logs.get('burns', [])) > 0
        has_trading_events = (
            len(parsed_logs.get('trading_enabled_events', [])) > 0 or 
            len(parsed_logs.get('trading_disabled_events', [])) > 0
        )
        
        # Check if this is a contract interaction
        input_data = transaction.get('input', '0x')
        is_contract_interaction = len(input_data) > 10  # More than just the function selector
        
        # Return true only if it's a pure transfer
        return not any([
            has_swap,
            has_pair,
            has_mint,
            has_burn,
            has_trading_events,
            is_contract_interaction
        ])
