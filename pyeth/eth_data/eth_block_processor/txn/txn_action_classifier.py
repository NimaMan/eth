from eth_utils import function_signature_to_4byte_selector
from web3 import Web3
from typing import Dict, Any, List

class TransactionActionClassifier:
    """
    Classifies transaction types by analyzing function signatures and logs
    """
    
    # Common function signatures
    SIGNATURES = {
        # Uniswap V2 Router
        "addLiquidityETH": "addLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
        "addLiquidity": "addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)",
        "removeLiquidity": "removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)",
        "removeLiquidityETH": "removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
        "swapExactTokensForTokens": "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
        "swapExactETHForTokens": "swapExactETHForTokens(uint256,address[],address,uint256)",
        "swapExactTokensForETH": "swapExactTokensForETH(uint256,uint256,address[],address,uint256)",
        
        # ERC20/ERC721
        "approve": "approve(address,uint256)",
        "transfer": "transfer(address,uint256)",
        "transferFrom": "transferFrom(address,address,uint256)",
    }

    def __init__(self):
        # Convert signatures to method IDs
        self.method_ids = {
            name: function_signature_to_4byte_selector(sig).hex()
            for name, sig in self.SIGNATURES.items()
        }
        
        # Reverse mapping from method ID to action type
        self.action_types = {
            self.method_ids["addLiquidityETH"]: "Add Liquidity",
            self.method_ids["addLiquidity"]: "Add Liquidity",
            self.method_ids["removeLiquidity"]: "Remove Liquidity",
            self.method_ids["removeLiquidityETH"]: "Remove Liquidity",
            self.method_ids["swapExactTokensForTokens"]: "Swap",
            self.method_ids["swapExactETHForTokens"]: "Swap",
            self.method_ids["swapExactTokensForETH"]: "Swap",
            self.method_ids["approve"]: "Approve",
            self.method_ids["transfer"]: "Transfer",
            self.method_ids["transferFrom"]: "Transfer",
        }

    def classify_transaction_action(self, 
                                 transaction: Dict[str, Any],
                                 parsed_logs: Dict[str, List[Any]]) -> str:
        """
        Determine the transaction type based on:
        1. Function signature in input data
        2. Event logs if no matching signature
        """
        # Check input data for function signature
        input_data = transaction.get('input', '0x')
        if len(input_data) >= 10:
            method_id = input_data[:10]  # includes '0x' prefix
            if method_id in self.action_types:
                return self.action_types[method_id]
        
        # Fallback to log analysis
        if self._is_add_liquidity_action(parsed_logs):
            return "Add Liquidity"
        elif self._is_swap_action(parsed_logs):
            return "Swap"
            
        return "Unknown"
    
    def _is_add_liquidity_action(self, parsed_logs: Dict[str, List[Any]]) -> bool:
        """Check for add liquidity pattern in logs"""
        has_sync = len(parsed_logs.get('uniswap_v2_syncs', [])) > 0
        has_mint = len(parsed_logs.get('mints', [])) > 0
        return has_sync and has_mint
    
    def _is_swap_action(self, parsed_logs: Dict[str, List[Any]]) -> bool:
        """Check for swap pattern in logs"""
        has_swap = len(parsed_logs.get('uniswap_v2_swaps', [])) > 0
        return has_swap