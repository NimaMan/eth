"""Token-level governance and policy tracker.

Responsibility:

* Track contract-level toggles: trading enabled/disabled, tax updates,
  max-buy limits/ratios.
* Detect hidden-mint patterns so tokens can be marked as inactive scams.
* Provide summary snapshots so the orchestrator can persist/report the
  current governance state.
"""

from typing import Dict, Optional, Union

__all__ = ["TokenStateMonitor"]


class TokenStateMonitor:
    """Encapsulates governance-related token state."""

    def __init__(self, *, hidden_mint_threshold: float = 1 + 1e-2) -> None:
        self.hidden_mint_threshold = hidden_mint_threshold

        # Trading enablement
        self.trading_enabled: bool = False
        self.trading_enabled_block: Optional[int] = None
        self.trading_enabled_timestamp: Optional[int] = None
        self.trading_enabled_tx: Optional[str] = None
        self.trading_enabled_event_index: Optional[int] = None
        self.trading_enabled_event: Optional[Dict] = None

        # Tax / max buy events
        self.tax_event: Optional[Dict] = None
        self.tax_event_block: Optional[int] = None
        self.tax_event_tx: Optional[str] = None
        self.tax_event_index: Optional[int] = None
        self.tax_event_log_index: Optional[int] = None

        self.max_buy_limit: Optional[int] = None
        self.max_buy_limit_block: Optional[int] = None
        self.max_buy_limit_tx: Optional[str] = None
        self.max_buy_limit_index: Optional[int] = None
        self.max_buy_limit_log_index: Optional[int] = None

        self.max_buy_ratio: Optional[int] = None
        self.max_buy_ratio_block: Optional[int] = None
        self.max_buy_ratio_tx: Optional[str] = None
        self.max_buy_ratio_index: Optional[int] = None
        self.max_buy_ratio_log_index: Optional[int] = None

        # Scam state
        self.is_scam: bool = False
        self.scam_label: Optional[str] = None
        self.scam_block: Optional[int] = None
        self.scam_tx: Optional[str] = None

    def update_from_transaction(self, transaction: Dict) -> None:
        self._process_trading_events(transaction)
        self._process_tax_events(transaction)
        self._process_max_buy_limit_events(transaction)
        self._process_max_buy_ratio_events(transaction)

    def _process_trading_events(self, transaction: Dict) -> None:
        trading_events = transaction.get("trading_enabled_events") or []
        if not trading_events:
            return

        self.trading_enabled_event = trading_events[-1]
        for _event in trading_events:
            if not self.trading_enabled:
                self._mark_trading_enabled(transaction)

    def _mark_trading_enabled(self, transaction: Dict) -> None:
        self.trading_enabled = True
        self.trading_enabled_block = transaction.get("block_number")
        self.trading_enabled_timestamp = transaction.get("block_timestamp")
        self.trading_enabled_tx = transaction.get("hash")
        self.trading_enabled_event_index = transaction.get("tx_index")

    def _process_tax_events(self, transaction: Dict) -> None:
        tx_hash = transaction.get("hash")
        block_number = transaction.get("block_number")
        tx_index = transaction.get("tx_index")

        for tax_event in transaction.get("tax_events", []):
            self.tax_event = tax_event
            self.tax_event_block = block_number
            self.tax_event_tx = tx_hash
            self.tax_event_index = tx_index
            self.tax_event_log_index = tax_event.get("log_index")

    def _process_max_buy_limit_events(self, transaction: Dict) -> None:
        tx_hash = transaction.get("hash")
        block_number = transaction.get("block_number")
        tx_index = transaction.get("tx_index")

        for event in transaction.get("max_buy_limit_events", []):
            self.max_buy_limit = event.get("max_buy_limit")
            self.max_buy_limit_block = block_number
            self.max_buy_limit_tx = tx_hash
            self.max_buy_limit_index = tx_index
            self.max_buy_limit_log_index = event.get("log_index")

    def _process_max_buy_ratio_events(self, transaction: Dict) -> None:
        tx_hash = transaction.get("hash")
        block_number = transaction.get("block_number")
        tx_index = transaction.get("tx_index")

        for event in transaction.get("max_buy_ratio_events", []):
            self.max_buy_ratio = event.get("max_buy_ratio")
            self.max_buy_ratio_block = block_number
            self.max_buy_ratio_tx = tx_hash
            self.max_buy_ratio_index = tx_index
            self.max_buy_ratio_log_index = event.get("log_index")

    # ------------------------------------------------------------------
    # Hidden mint detection
    # ------------------------------------------------------------------

    def detect_hidden_mint(
        self,
        *,
        total_supply: Optional[float],
        total_supply_from_transfers: Optional[float],
        transaction: Dict,
    ) -> bool:
        """Return True if hidden-mint conditions were triggered."""
        if float(total_supply_from_transfers) > float(total_supply) * self.hidden_mint_threshold:
            self.mark_scam(
                label="hidden_mint",
                block_number=transaction.get("block_number"),
                tx_hash=transaction.get("hash"),
            )
            return True

        return False

    def mark_scam(self, *, label: str, block_number: Optional[int], tx_hash: Optional[str]) -> None:
        """Record scam metadata if we have not already done so."""

        if self.is_scam and self.scam_label == label:
            return

        self.is_scam = True
        self.scam_label = label
        self.scam_block = block_number
        self.scam_tx = tx_hash

    def clear_scam_flag(self) -> None:
        """Reset scam metadata (used when pools recover)."""

        self.is_scam = False
        self.scam_label = None
        self.scam_block = None
        self.scam_tx = None

    def build_state_snapshot(self) -> Dict[str, Optional[Union[int, str, float]]]:
        """Return a dict containing the health-related fields."""

        return {
            "trading_enabled": self.trading_enabled,
            "trading_enabled_block": self.trading_enabled_block,
            "trading_enabled_timestamp": self.trading_enabled_timestamp,
            "trading_enabled_tx": self.trading_enabled_tx,
            "trading_enabled_event_index": self.trading_enabled_event_index,
            "trading_enabled_event": self.trading_enabled_event,
            "tax_event": self.tax_event,
            "tax_event_block": self.tax_event_block,
            "tax_event_tx": self.tax_event_tx,
            "tax_event_index": self.tax_event_index,
            "tax_event_log_index": self.tax_event_log_index,
            "max_buy_limit": self.max_buy_limit,
            "max_buy_limit_block": self.max_buy_limit_block,
            "max_buy_limit_tx": self.max_buy_limit_tx,
            "max_buy_limit_index": self.max_buy_limit_index,
            "max_buy_limit_log_index": self.max_buy_limit_log_index,
            "max_buy_ratio": self.max_buy_ratio,
            "max_buy_ratio_block": self.max_buy_ratio_block,
            "max_buy_ratio_tx": self.max_buy_ratio_tx,
            "max_buy_ratio_index": self.max_buy_ratio_index,
            "max_buy_ratio_log_index": self.max_buy_ratio_log_index,
            "is_scam": self.is_scam,
            "scam_label": self.scam_label,
            "scam_block": self.scam_block,
            "scam_tx": self.scam_tx,
        }
