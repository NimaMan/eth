"""
ScamPredictor: Analyzes token transactions for scam patterns

Objective:
---------
1. Track known malicious actors' involvement
2. Detect deceptive trading patterns
3. Identify fake volume through transfers to legitimate actors
4. Provide real-time scam probability scoring

1. Malicious Actor Tracking:
   - Maintains list of known scammer addresses
   - Tracks interactions with token
   - Monitors transfer patterns

"""

from typing import Dict, Set, Optional, List
from dataclasses import dataclass, asdict
from eth_tokens_live.live_erc20_token.scam_pred.deceptive_volume_checker import DeceptiveVolumeChecker


@dataclass
class ScamScore:
    is_scam: bool
    confidence: float
    reason: str
    block_number: int
    transaction_hash: str


class TokenHealthPredictor:
    def __init__(self):
        self.deceptive_volume_checker = DeceptiveVolumeChecker()
        self.scam_scores: List[ScamScore] = []
        self.involved_mal_actors: Set[str] = set()
        self.involved_green_actors: Set[str] = set()
        self.abused_green_actors: Set[str] = set()
        
    def update_from_transaction(self, transaction: Dict) -> Dict:
        """Process new transaction for scam detection"""
        try:
            green_actors = self.deceptive_volume_checker.green_actors_involved(transaction)
            if green_actors:
                # Check if it is  a fake buy for deceptive volume
                if self._check_fake_buy(transaction, green_actors):
                    return self.token_health_assessment
                else:
                    self.involved_green_actors.update(green_actors)
                    return self.token_health_assessment
            # Check if it is  a malicious swap for deceptive volume
            self._check_malicious_swap(transaction)
            return self.token_health_assessment
        
        except Exception as e:
            raise Exception(f"{self.__class__.__name__} Error processing transaction: {e}")    

    def _check_fake_buy(self, transaction: Dict, green_actors: Set[str]) -> bool:
        """Check for fake buy pattern and update scores if detected"""
        if self.deceptive_volume_checker.is_fake_buy(transaction):
            self.scam_scores.append(ScamScore(
                is_scam=True,
                confidence=1,
                reason="Fake Volume detected",
                block_number=transaction.get('block_number'),
                transaction_hash=transaction.get('hash')
            ))
            self.abused_green_actors.update(green_actors)
            return True
        return False

    def _check_malicious_swap(self, transaction: Dict) -> bool:
        """Check for malicious swap pattern and update scores if detected"""
        mal_actors = self.deceptive_volume_checker.malicious_actor_swap(transaction)
        if mal_actors:
            self.involved_mal_actors.update(mal_actors)
            self.scam_scores.append(ScamScore(
                is_scam=True,
                confidence=.95,
                reason="Malicious actors involved",
                block_number=transaction.get('block_number'),
                transaction_hash=transaction.get('hash')
            ))
            return True
        return False
    
    @property
    def is_scam(self) -> bool:
        """Return whether token is currently flagged as scam"""
        return any(score.is_scam for score in self.scam_scores)
    
    @property
    def scam_confidence(self) -> float:
        """Return confidence in scam classification"""
        return max(score.confidence for score in self.scam_scores) if self.scam_scores else 0.0
    
    @property
    def scam_reason(self) -> str:
        """Return reason for scam classification"""
        return set(score.reason for score in self.scam_scores) if self.scam_scores else "Not analyzed"
    
    @property
    def scam_detection_block_and_txn(self):
        """Return block number and transaction hash where scam was detected"""
        return (self.scam_scores[0].block_number, self.scam_scores[0].transaction_hash) if self.scam_scores else None
    
    @property
    def num_involved_green_actors(self) -> int:
        """Return number of green actors involved"""
        return len(self.involved_green_actors)
    
    @property
    def num_involved_mal_actors(self) -> int:
        """Return number of mal actors involved"""
        return len(self.involved_mal_actors)
    
    @property
    def scam_assessment(self) -> Dict:
        """Return assessment of token health"""
        return {
            "is_scam": self.is_scam,
            "confidence": self.scam_confidence,
            "reason": self.scam_reason,
            "block_number": self.scam_detection_block_and_txn[0] if self.scam_detection_block_and_txn else None,
            "transaction_hash": self.scam_detection_block_and_txn[1] if self.scam_detection_block_and_txn else None,
            'involved_addresses': list(self.involved_mal_actors | self.abused_green_actors),
        }
    
    @property
    def green_assessment(self) -> Dict:
        """Return assessment of green actors"""
        return {
            "green_actors": self.involved_green_actors,
        }
    
    @property
    def token_health_assessment(self) -> Dict:
        """Return assessment of token health"""
        return {
            'scam_assessment': self.scam_assessment,
            'green_assessment': self.green_assessment,
        }

    def to_dict(self):
        return self.token_health_assessment