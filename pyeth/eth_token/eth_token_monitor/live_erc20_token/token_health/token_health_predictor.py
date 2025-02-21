"""
TokenHealthPredictor: Analyzes token transactions for health patterns

Objective:
---------
1. Track known malicious actors' involvement
2. Detect deceptive trading patterns
3. Identify fake volume through transfers to legitimate actors
4. Provide real-time health probability scoring

1. Malicious Actor Tracking:
   - Maintains list of known scammer addresses
   - Tracks interactions with token
   - Monitors transfer patterns

"""

from typing import Dict, Set, Optional, List
from dataclasses import dataclass, asdict
from collections import OrderedDict
from eth_token_monitor.live_erc20_token.token_health.volume_analyzer import VolumeAnalyzer


@dataclass
class ScamScore:
    block_number: int
    transaction_hash: str
    from_address: str
    is_scam: bool
    confidence: float
    reason: str
    
    
class TokenHealthPredictor:
    def __init__(self):
        self.volume_analyzr = VolumeAnalyzer()
        self.scam_scores: OrderedDict[str, ScamScore] = OrderedDict()
        self.involved_green_actors: OrderedDict[str, ScamScore] = OrderedDict() # txn hash -> green actors
        self.involved_mal_actors: Set[str] = set()
        
    def update_from_transaction(self, transaction: Dict, live_token) -> Dict:
        """Process new transaction for scam detection"""
        try:
            self._check_green_actors_involved(transaction)
            # Check if it is  a deceptive transfer
            self._check_deceptive_transfer(transaction)
            # Check if it is  a malicious swap for deceptive volume from grey addresses
            self._check_malicious_swap(transaction)
            self._check_token_scam_label(transaction, live_token)
            return self.token_health_assessment
        except Exception as e:
            raise Exception(f"{self.__class__.__name__} Error processing transaction: {e}")    

    def _check_green_actors_involved(self, transaction: Dict) -> bool:
        """Check if the transaction involves green actors"""
        green_actors = self.volume_analyzr.get_green_actors_involved(transaction)
        if green_actors:
            self.involved_green_actors[transaction.get('hash')] = green_actors

    def _check_deceptive_transfer(self, transaction: Dict, num_transfers_threshold: int = 15, num_addresses_threshold: int = 20) -> bool:
        """Check for deceptive transfer pattern and update scores if detected"""
        num_erc20_transfers, num_unique_addresses = self.volume_analyzr.get_num_transfers_and_addresses(transaction)
        if num_erc20_transfers and num_unique_addresses:
            if num_erc20_transfers > num_transfers_threshold or num_unique_addresses > num_addresses_threshold:
                self.scam_scores[transaction.get('hash')] = ScamScore(
                    block_number=transaction.get('block_number'),
                    transaction_hash=transaction.get('hash'),
                    from_address=transaction.get('from_address'),
                    is_scam=True,
                    confidence=1,
                    reason=f"{num_erc20_transfers} transfers and {num_unique_addresses} unique addresses",
                )

    def _check_malicious_swap(self, transaction: Dict) -> bool:
        """Check for malicious swap pattern and update scores if detected"""
        mal_actors = self.volume_analyzr.get_malicious_actor_swap(transaction)
        if mal_actors:
            self.involved_mal_actors.update(mal_actors)
            self.scam_scores[transaction.get('hash')] = ScamScore(
                block_number=transaction.get('block_number'),
                transaction_hash=transaction.get('hash'),
                from_address=transaction.get('from_address'),
                is_scam=True,
                confidence=.95,
                reason="Malicious actors involved",
            )

    def _check_token_scam_label(self, transaction: Dict, live_token) -> bool:
        """Check if the token has a scam label"""
        if live_token.is_scam:
            self.scam_scores[transaction.get('hash')] = ScamScore(
                block_number=transaction.get('block_number'),
                transaction_hash=transaction.get('hash'),
                from_address=transaction.get('from_address'),
                is_scam=True,
                confidence=1,
                reason=f"{live_token.scam_label}",
            )
    
    @property
    def is_scam(self) -> bool:
        """Return whether token is currently flagged as scam"""
        return any(score.is_scam for score in self.scam_scores.values())
    
    @property
    def scam_probability(self) -> float:
        """Return probability of scam"""
        return max(score.confidence for score in self.scam_scores.values()) if self.scam_scores else 0.0
    
    @property
    def scam_reason(self) -> str:
        """Return reason for scam classification"""
        return tuple(set(score.reason for score in self.scam_scores.values())) if self.scam_scores else "NA"
    
    @property
    def scam_detection_block_and_txn(self):
        """Return block number and transaction hash where scam was detected"""
        if not self.scam_scores:
            return None
        last_hash = next(reversed(self.scam_scores))
        last_score = self.scam_scores[last_hash]
        return (last_score.block_number, last_score.transaction_hash)
    
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
            "scam_probability": self.scam_probability,
            "scam_reason": self.scam_reason,
            "block_number": self.scam_detection_block_and_txn[0] if self.scam_detection_block_and_txn else None,
            "transaction_hash": self.scam_detection_block_and_txn[1] if self.scam_detection_block_and_txn else None,
            'involved_grey_addresses': list(self.involved_mal_actors),
            'num_greys': len(self.involved_mal_actors),
        }
    
    @property
    def green_assessment(self) -> Dict:
        """Return assessment of green actors"""
        return {
            "green_actors": self.involved_green_actors,
            'num_greens': len(self.involved_green_actors),
        }
    
    @property
    def token_health_assessment(self) -> Dict:
        """Return assessment of token health"""
        return {
            **self.scam_assessment,
            **self.green_assessment,
        }

    def to_dict(self):
        return self.token_health_assessment