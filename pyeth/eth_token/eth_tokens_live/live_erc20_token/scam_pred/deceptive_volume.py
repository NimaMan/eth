"""
DeceptiveVolume: Analyzes token transfers for deceptive patterns

Objective:
---------
Detect 

Architecture:
-----------
1. Volume Tracking:
   - Genuine trading volume
   - Artificial/deceptive volume
   - Transfer patterns to legitimate actors
   
2. Pattern Detection:
   - Wash trading detection
   - Circular transfer patterns
   - Suspicious timing patterns
"""

from typing import Dict, Set
from dataclasses import dataclass
from eth_tokens_live.alert.config import get_green_addresses


@dataclass
class VolumeMetrics:
    genuine_volume: float = 0.0
    deceptive_volume: float = 0.0
    wash_trade_count: int = 0
    last_block: int = 0


class DeceptiveVolume:
    def __init__(self):
        self.green_addresses: Set[str] = get_green_addresses()
        self.metrics = VolumeMetrics()
        self.suspicious_transfers: Dict[str, float] = {}  # address -> volume
        
    def process_transaction(self, transaction: Dict, has_mal_actor: bool) -> None:
        """Process transaction for deceptive volume detection"""
        try:
            if "Swap" not in transaction.get('action', ''):
                return
                
            volume = float(transaction.get('value', 0))
            involved_addresses = set(transaction.get('unique_addresses', []))
            
            # Check if transaction involves legitimate actors
            has_green_actor = bool(self.green_addresses & involved_addresses)
            
            if has_green_actor:
                if has_mal_actor:
                    # Volume from malicious actors to legitimate actors
                    self.metrics.deceptive_volume += volume
                else:
                    # Genuine trading volume
                    self.metrics.genuine_volume += volume
                    
            # Update last processed block
            self.metrics.last_block = transaction.get('block_number', self.metrics.last_block)
            
        except Exception as e:
            raise Exception(f"Error processing transaction for deceptive volume: {e}")
    
    @property
    def deceptive_ratio(self) -> float:
        """Calculate ratio of deceptive to total volume"""
        total_volume = self.metrics.genuine_volume + self.metrics.deceptive_volume
        return self.metrics.deceptive_volume / total_volume if total_volume > 0 else 0.0
    
    @property
    def is_suspicious(self) -> bool:
        """Check if token shows suspicious volume patterns"""
        return self.deceptive_ratio > 0.5
    
    @property
    def detection_block(self) -> int:
        """Get block number of latest analysis"""
        return self.metrics.last_block 