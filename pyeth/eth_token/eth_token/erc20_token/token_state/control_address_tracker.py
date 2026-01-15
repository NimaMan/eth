"""Control-address tracking helpers.

Responsibility:

* Maintain the canonical set of addresses that are allowed to exercise
  privileged paths on the token contract (enabling trading, mutating
  taxes/fees, draining liquidity, pausing, etc.).
* React to every on-chain mechanism that can grant or transition that
  authority:
  - Contract creation (creator + initial owner) and any subsequent
    ``OwnershipTransferred`` events, including the Ownable2Step
    ``OwnershipTransferStarted`` pattern.
  - AccessControl ``RoleGranted``/``RoleRevoked`` events for the known
    administrator roles (default admin, controller, pauser, etc.) that
    expose privileged methods.
  - Transparent proxy ``AdminChanged`` events so proxy admins are treated
    as controllers.
  - Optional manual seeds (e.g., metadata-supplied control addresses)
    provided at instantiation time.
* Provide a simple API that reports new control addresses so that the
  pool manager, simulators, and any downstream consumers stay perfectly
  in sync with the token’s latest owner/controller set.
"""

from typing import Dict, Iterable, List, Optional, Sequence, Set, Union
from web3 import Web3

from eth_token.utils import bounded_history


__all__ = [
    "DEFAULT_ADMIN_ROLE_SIGNATURE",
    "ACCESS_CONTROL_CONTROL_ROLE_NAMES",
    "ACCESS_CONTROL_CONTROL_ROLE_SIGNATURES",
    "ControlAddressTracker",
]

DEFAULT_ADMIN_ROLE_SIGNATURE = "0x" + ("0" * 64)

ACCESS_CONTROL_CONTROL_ROLE_NAMES: Sequence[str] = (
    "DEFAULT_ADMIN_ROLE",
    "ADMIN_ROLE",
    "CONTROLLER_ROLE",
    "OWNER_ROLE",
    "MANAGER_ROLE",
    "PAUSER_ROLE",
    "GOVERNANCE_ROLE",
    "TRADING_ROLE",
)


def _default_role_signatures() -> Set[str]:
    """Return the canonical set of AccessControl role hashes with control powers."""

    signatures: Set[str] = {DEFAULT_ADMIN_ROLE_SIGNATURE.lower()}
    for role_name in ACCESS_CONTROL_CONTROL_ROLE_NAMES:
        signatures.add(Web3.keccak(text=role_name).hex().lower())
    return signatures


class ControlAddressTracker:
    """Tracks which addresses currently control token-level authority.

    Parameters
    ----------
    initial_addresses:
        Optional iterable of addresses to seed the control set with
        (e.g., creator, metadata-provided controllers).
    additional_role_signatures:
        Optional iterable of AccessControl role hashes that should also
        grant control privileges beyond the built-in defaults.
    """

    def __init__(
        self,
        *,
        history_limit: int = 1000,
        initial_addresses: Optional[Iterable[Optional[str]]] = None,
        additional_role_signatures: Optional[Iterable[str]] = None,
    ) -> None:
        self.token_control_addresses: Set[str] = set()
        self._role_signatures: Set[str] = _default_role_signatures()
        self.history_limit = int(history_limit)
        self.all_owners: List[Optional[str]] = []
        self.owner_events: List[Dict] = []
        self.current_owner: Optional[str] = None
        self.ownership_renounced: bool = False
        self.renouncement_block: Optional[int] = None
        self.renouncement_tx: Optional[str] = None
        self.renouncement_event: Optional[Dict] = None
        self.renouncement_event_index: Optional[int] = None
        self.renouncement_event_log_index: Optional[int] = None
        if additional_role_signatures:
            for signature in additional_role_signatures:
                normalized = self._normalize_role_signature(signature)
                if normalized:
                    self._role_signatures.add(normalized)

        if initial_addresses:
            self.register(initial_addresses)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    @property
    def addresses(self) -> Set[str]:
        """Return all currently known control addresses (checksum format)."""

        return set(self.token_control_addresses)

    def register(self, addresses: Iterable[Optional[str]]) -> List[str]:
        """Add addresses to the control set.

        Returns a list of addresses that were actually added (useful to
        keep downstream systems in sync without deduplicating manually).
        """

        newly_added: List[str] = []
        for address in addresses:
            if not address:
                continue
            if address not in self.token_control_addresses:
                self.token_control_addresses.add(address)
                newly_added.append(address)
        return newly_added

    def update_from_transaction(self, transaction: Dict) -> List[str]:
        """Process a processed-transaction dict and return any new controllers."""

        newly_added: List[str] = []
        newly_added.extend(self._handle_owner_events(transaction))
        newly_added.extend(
            self._handle_pending_owner_events(
                transaction.get("ownership_transfer_started_events")
            )
        )
        newly_added.extend(
            self._handle_access_control_role_events(
                transaction.get("access_control_role_granted_events"),
                transaction.get("access_control_role_revoked_events"),
            )
        )
        newly_added.extend(
            self._handle_proxy_admin_events(transaction.get("proxy_admin_changed_events"))
        )
        self._handle_renouncement_events(transaction)
        return newly_added

    # ------------------------------------------------------------------
    # Event handlers
    # ------------------------------------------------------------------

    def _handle_owner_events(self, transaction: Dict) -> List[str]:
        owner_events = transaction.get("owner_events") or []
        if not owner_events:
            return []
        new_owner_addresses = [event.get("new_owner") for event in owner_events]
        newly_added = self.register(new_owner_addresses)

        block_number = transaction.get("block_number")
        tx_hash = transaction.get("hash")
        tx_index = transaction.get("tx_index")

        for event in owner_events:
            new_owner = event.get("new_owner")
            previous_owner = event.get("previous_owner")

            if new_owner:
                bounded_history.append_with_history_limit(
                    self.all_owners, new_owner, self.history_limit
                )
                self.current_owner = new_owner

            if previous_owner == "0x0000000000000000000000000000000000000000":
                self.ownership_renounced = True
                self.renouncement_block = block_number
                self.renouncement_tx = tx_hash

            bounded_history.append_with_history_limit(
                self.owner_events,
                {
                    "tx_hash": tx_hash,
                    "block_number": block_number,
                    "tx_index": tx_index,
                    "log_index": event.get("log_index"),
                    "previous_owner": previous_owner,
                    "new_owner": new_owner,
                    "token_address": event.get("contract_address"),
                },
                self.history_limit,
            )

        return newly_added

    def _handle_pending_owner_events(
        self, pending_events: Optional[Sequence[Dict]]
    ) -> List[str]:
        if not pending_events:
            return []
        pending = [event.get("new_owner") for event in pending_events]
        return self.register(pending)

    def _handle_access_control_role_events(
        self,
        granted_events: Optional[Sequence[Dict]],
        revoked_events: Optional[Sequence[Dict]],  # noqa: ARG002
    ) -> List[str]:
        if not granted_events:
            return []
        new_accounts: List[str] = []
        for event in granted_events:
            role = event.get("role")
            if not self._role_grants_control(role):
                continue
            account = event.get("account")
            new_accounts.extend(self.register([account]))
        # Revocations are ignored for now—removing addresses could desync
        # pools if we miss historical context. Implemented behaviour
        # matches the previous ERC20TokenData logic.
        return new_accounts

    def _handle_proxy_admin_events(
        self, admin_events: Optional[Sequence[Dict]]
    ) -> List[str]:
        if not admin_events:
            return []
        return self.register(event.get("new_admin") for event in admin_events)

    def _handle_renouncement_events(self, transaction: Dict) -> None:
        renouncement_events = transaction.get("renouncement_events")
        if not renouncement_events:
            return

        tx_hash = transaction.get("hash")
        block_number = transaction.get("block_number")
        tx_index = transaction.get("tx_index")

        for event in renouncement_events:
            self.ownership_renounced = True
            self.renouncement_block = block_number
            self.renouncement_tx = tx_hash
            self.renouncement_event = event
            self.renouncement_event_index = tx_index
            self.renouncement_event_log_index = event.get("log_index")

    # ------------------------------------------------------------------
    # Introspection helpers
    # ------------------------------------------------------------------

    def get_owner_history(self) -> List[Optional[str]]:
        return list(self.all_owners)

    def get_owner_events(self) -> List[Dict]:
        return list(self.owner_events)

    def get_renouncement_state(self) -> Dict[str, Optional[Union[int, str, Dict]]]:
        return {
            "ownership_renounced": self.ownership_renounced,
            "renouncement_block": self.renouncement_block,
            "renouncement_tx": self.renouncement_tx,
            "renouncement_event": self.renouncement_event,
            "renouncement_event_index": self.renouncement_event_index,
            "renouncement_event_log_index": self.renouncement_event_log_index,
        }

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------
    @staticmethod
    def _normalize_role_signature(role: Optional[str]) -> Optional[str]:
        if not role:
            return None
        role_signature = role.lower()
        if not role_signature.startswith("0x"):
            role_signature = f"0x{role_signature}"
        return role_signature

    def _role_grants_control(self, role: Optional[str]) -> bool:
        normalized = self._normalize_role_signature(role)
        if not normalized:
            return False
        return normalized in self._role_signatures
