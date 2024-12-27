"""
MaliciousActor: Tracks known malicious addresses and their interactions

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
from eth_tokens_live.alert.config import get_grey_addresses


class MaliciousActor:
    def __init__(self):
        # Initialize address sets
        self.grey_addresses_set: Set[str] = get_grey_addresses()
        
        # Track current malicious actors
        self.current_mal_actors: Set[str] = set()
        self.first_seen_block: Dict[str, int] = {}
        
    def process_txn(self, transaction: Dict) -> None:
        """Process transaction for malicious actor detection"""
        try:
            block_number = transaction.get('block_number')
            involved_addresses = set(transaction.get('unique_addresses', []))
            
            # Check for new malicious actors
            new_mal_actors = self.grey_addresses_set & involved_addresses
            
            # Update tracking
            for actor in new_mal_actors:
                if actor not in self.current_mal_actors:
                    self.first_seen_block[actor] = block_number
                    
            self.current_mal_actors.update(new_mal_actors)
            
        except Exception as e:
            raise Exception(f"Error processing transaction for malicious actors: {e}")
    
    def has_malicious_actor(self, transaction: Dict) -> bool:
        """Check if transaction involves known malicious actors"""
        involved_addresses = set(transaction.get('unique_addresses', []))
        return bool(self.grey_addresses_set & involved_addresses)
    
    @property
    def is_compromised(self) -> bool:
        """Check if token has been involved with malicious actors"""
        return len(self.current_mal_actors) > 0
    
    @property
    def first_mal_actor_block(self) -> int:
        """Get block number when first malicious actor was detected"""
        return min(self.first_seen_block.values()) if self.first_seen_block else 0
    
    @property
    def mal_actor_addresses(self) -> Set[str]:
        """Get set of malicious actors involved with token"""
        return self.current_mal_actors.copy()

    @property
    def num_mal_actors(self) -> int:
        """Get number of malicious actors involved with token"""
        return len(self.current_mal_actors)
