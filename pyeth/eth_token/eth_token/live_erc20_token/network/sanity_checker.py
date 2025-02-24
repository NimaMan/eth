"""
# Token Network Sanity Checks
Validates the integrity and consistency of token transactions and balances.

1. Token Movement Validation:
   - Excess Sales Check
     * No address should sell more than bought unless it is a scam token
     * Exception: Zero address and scam tokens
     * Tolerance: < 1e-1 for floating point arithmetic

2. Balance Conservation:
   - Negative Balance Check
     * No address should have negative token balance
     * Exceptions:
       - Zero address (0x000...000)
       - Scam tokens with hidden minting
     * Tolerance: < 1e-1 for floating point arithmetic
"""

import networkx as nx
import numpy as np
import pandas as pd


class NetworkSanityChecker:
    def __init__(self, logger, graph: nx.DiGraph, token_data, denom_balance_tolerance=1e-3, token_balance_tolerance=1e-1):
        self.logger = logger
        self.graph = graph
        self.token_data = token_data
        self.denom_balance_tolerance = denom_balance_tolerance
        self.token_balance_tolerance = token_balance_tolerance
        self.zero_address = "0x0000000000000000000000000000000000000000"
        self.scammer_address = None
        self.total_token_supply = self.token_data.total_supply
        self.is_hidden_minting = False
        self.is_scam = token_data.is_scam  # Add scam token flag
        self.scam_label = token_data.scam_label  # Add scam label
        self.check_sanity()
        
    def check_sanity(self) -> bool:
        """Comprehensive sanity check of token network with detailed logging."""
        # Check each node in the network
        for node in self.graph:
            node_data = self.graph.nodes[node]['data']
            total_token_bought = node_data.total_token_bought
            total_token_sold = node_data.total_token_sold
            # check excess sales
            self._check_excess_sales(node, total_token_bought, total_token_sold)
            # Check negative token balances
            self._check_negative_balance(node, node_data.token_balance)
            
    def _check_negative_balance(self, node: str, token_balance: float) -> bool:
        """Check for negative token balances with scam token awareness"""
        
        if token_balance < -self.token_balance_tolerance:
            # Skip validation for zero address
            if node == self.zero_address:
                return True
            # For scam tokens, only log hidden minting detection
            if self.is_scam:
                if abs(token_balance)/self.total_token_supply > 1:
                    self.is_hidden_minting = True
                    self.scammer_address = node
                return True  # Don't report as error for scam tokens
                
            # Normal token validation
            self.logger.info(
                f"Address {node} has negative token balance: {token_balance} "
                f"Contract: {self.token_data.contract_address}"
            )
            return False
        return True

    def _check_excess_sales(self, node: str, total_bought: float, total_sold: float) -> bool:
        """Check for excess token sales and log violations"""
        if np.abs(total_sold - total_bought) > self.token_balance_tolerance:
            if node == self.zero_address and self.is_scam:
                return True
            self.logger.info(
                f"Address {node} selling more than bought: "
                f"sold={total_sold} bought={total_bought} "
                f"Contract: {self.token_data.contract_address}"
            )
            return False
        return True
 