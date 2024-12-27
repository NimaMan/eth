"""
ScamPredictor: Analyzes token transactions for scam patterns

Objective:
---------
1. Track known malicious actors' involvement
2. Detect deceptive trading patterns
3. Identify fake volume through transfers to legitimate actors
4. Provide real-time scam probability scoring

Architecture:
-----------
1. Malicious Actor Tracking:
   - Maintains list of known scammer addresses
   - Tracks interactions with token
   - Monitors transfer patterns

2. Deceptive Volume Analysis:
   - Tracks transfers to legitimate actors
   - Identifies wash trading patterns
   - Calculates genuine vs artificial volume
"""

from typing import Dict, Set, Optional
from dataclasses import dataclass
from eth_tokens_live.live_erc20_token.scam_pred.mal_actors import MaliciousActor


@dataclass
class ScamScore:
    is_scam: bool
    confidence: float
    reason: str
    block_number: int
    transaction_hash: str


class ScamPredictor:
    def __init__(self):
        self.mal_actors = MaliciousActor()
        self.scam_score: Optional[ScamScore] = None
        self.deceptive_volume = 0.0
        self.genuine_volume = 0.0
        
    def process_transaction(self, transaction: Dict) -> None:
        """Process new transaction for scam detection"""
        try:
            # Check for malicious actors
            mal_actor_alerts = self.mal_actors.process_txn(transaction)
            
            # Update volumes
            if self.mal_actors.has_green_actor(transaction):
                if "Swap" in transaction.get('action', ''):
                    # Potential deceptive volume
                    if self.mal_actors.has_malicious_actor(transaction):
                        self.deceptive_volume += float(transaction.get('value', 0))
                    else:
                        self.genuine_volume += float(transaction.get('value', 0))
            
            # Update scam score
            self._update_scam_score(transaction)
            
        except Exception as e:
            raise Exception(f"Error processing transaction for scam detection: {e}")
    
    def _update_scam_score(self, transaction: Dict) -> None:
        """Update scam score based on latest transaction data"""
        block_number = transaction.get('block_number')
        tx_hash = transaction.get('hash')
        
        # Check for direct malicious actor involvement
        if self.mal_actors.current_mal_actors:
            self.scam_score = ScamScore(
                is_scam=True,
                confidence=0.9,
                reason="Known malicious actors involved",
                block_number=block_number,
                transaction_hash=tx_hash
            )
            return
            
        # Check for deceptive volume
        if self.deceptive_volume > 0:
            total_volume = self.deceptive_volume + self.genuine_volume
            if total_volume > 0:
                deceptive_ratio = self.deceptive_volume / total_volume
                if deceptive_ratio > 0.5:
                    self.scam_score = ScamScore(
                        is_scam=True,
                        confidence=min(deceptive_ratio, 0.9),
                        reason="High ratio of deceptive volume",
                        block_number=block_number,
                        transaction_hash=tx_hash
                    )
                    return
        
        # No scam indicators found
        self.scam_score = ScamScore(
            is_scam=False,
            confidence=0.7,
            reason="No scam indicators detected",
            block_number=block_number,
            transaction_hash=tx_hash
        )
    
    @property
    def is_scam(self) -> bool:
        """Return whether token is currently flagged as scam"""
        return self.scam_score.is_scam if self.scam_score else False
    
    @property
    def scam_confidence(self) -> float:
        """Return confidence in scam classification"""
        return self.scam_score.confidence if self.scam_score else 0.0
    
    @property
    def scam_reason(self) -> str:
        """Return reason for scam classification"""
        return self.scam_score.reason if self.scam_score else "Not analyzed"
    
    @property
    def detection_block(self) -> Optional[int]:
        """Return block number where scam was detected"""
        return self.scam_score.block_number if self.scam_score else None
    
    @property
    def detection_tx(self) -> Optional[str]:
        """Return transaction hash where scam was detected"""
        return self.scam_score.transaction_hash if self.scam_score else None
