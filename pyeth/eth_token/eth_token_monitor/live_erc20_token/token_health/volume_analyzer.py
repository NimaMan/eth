"""
DeceptiveVolumeChecker: Tracks known malicious addresses and their interactions

Objective:
---------
1. Track known malicious addresses
2. Monitor their interactions with tokens
3. Detect when malicious actors are involved in transactions
4. Track interactions between malicious and legitimate actors

Architecture:
-----------
1. Address Lists:
   - Grey list: Known malicious addresses
   - Green list: Known legitimate addresses (whales, oracles, etc.)
   
2. Tracking:
   - Current malicious actors involved with token
   - Transaction history with malicious actors
   - Interaction patterns
"""

from typing import List, Set, Dict
from eth_token_monitor.alert.config import get_mimic_octopus_addresses, get_green_addresses


class VolumeAnalyzer:
    """
    DeceptiveVolume: Analyzes token transfers for deceptive patterns
    ---------
    Detect Deceptive volume created by
        - Genuine trades from malicious actors (grey addresses)
        - Token transfers to legimiate actors that are not swaps (real buys)    
    """

    def __init__(self):
        self.green_addresses: Set[str] = get_green_addresses()
        self.mimic_octopus_addresses: Set[str] = get_mimic_octopus_addresses()
        
    def get_malicious_actor_swap(self, transaction: Dict):
        """Check if transaction is a malicious swap with legitimate actors"""   
        involved_addresses = set(transaction.get('unique_addresses', []))
        mal_actors = self.mimic_octopus_addresses & involved_addresses
        if len(mal_actors) > 1:
            return mal_actors
        return None

    def get_green_actors_involved(self, transaction: Dict):
        """Check if transaction is a fake buy (transfer without swap)"""
        involved_addresses = set(transaction.get('unique_addresses', []))
        green_actors = self.green_addresses & involved_addresses
        if len(green_actors) > 1:
            return green_actors
        return None
    
    def get_num_transfers_and_addresses(self, transaction: Dict):
        """Get the number of transfers and addresses in a transaction"""                
        # Check for high number of transfers or addresses (suspicious pattern)
        num_erc20_transfers = len(transaction.get('erc20_transfers', []))
        num_unique_addresses = len(transaction.get('unique_addresses', []))
        return num_erc20_transfers, num_unique_addresses

