"""Token-level transfer and approval tracker.

Responsibility:

* Append ERC20 transfers, denomination transfers, and internal ETH
  movements into bounded history structures.
* Maintain approval history plus the ``approved_addresses`` summary set.
* Update per-address counters used by analytics, token-network
  generation, and health checks.

All logic mirrors the helper methods that currently live inside
``ERC20TokenData``.  Once the orchestrator is wired to this module the
behaviour should remain bit-for-bit identical.
"""

from typing import Dict, Iterable, List, MutableMapping, Optional, Set

from eth_token.erc20_token.pools.pool_manager import UNISWAP_V2_PROTOCOL
from eth_token.utils import bounded_history
from eth_data.chain_utils.common_addresses import (
    DENOM_ADDRESSES,
    DENOM_NAMES_TO_ADDRESS,
    ERC20_TOKEN_DECIMALS,
)

__all__ = ["TokenTransferTracker"]


class TokenTransferTracker:
    """Encapsulates transfer/approval bookkeeping for a single token."""

    def __init__(
        self,
        *,
        contract_address: str,
        decimals: int,
        history_limit: int,
        pool_manager=None,
        erc20_transfers: Optional[MutableMapping[str, List[Dict]]] = None,
        eth_transfers: Optional[MutableMapping[str, List[Dict]]] = None,
        other_denom_transfers: Optional[MutableMapping[str, List[Dict]]] = None,
        approvals: Optional[List[Dict]] = None,
        approved_addresses: Optional[Set[str]] = None,
        other_currencies: Optional[Dict[str, int]] = None,
        address_tx_counter: Optional[Dict[str, int]] = None,
        total_supply_from_transfers: float = 0.0,
    ) -> None:
        self.contract_address = contract_address
        self.decimals = int(decimals)
        self.history_limit = int(history_limit)
        self.pool_manager = pool_manager

        self.erc20_transfers: MutableMapping[str, List[Dict]] = (
            erc20_transfers if erc20_transfers is not None else {}
        )
        self.eth_transfers: MutableMapping[str, List[Dict]] = (
            eth_transfers if eth_transfers is not None else {}
        )
        self.other_denom_transfers: MutableMapping[str, List[Dict]] = (
            other_denom_transfers if other_denom_transfers is not None else {}
        )
        self.approvals: List[Dict] = approvals if approvals is not None else []
        self.approved_addresses: Set[str] = (
            approved_addresses if approved_addresses is not None else set()
        )
        self.other_currencies: Dict[str, int] = (
            other_currencies if other_currencies is not None else {}
        )
        self.address_tx_counter: Dict[str, int] = (
            address_tx_counter if address_tx_counter is not None else {}
        )
        self.total_supply_from_transfers: float = float(total_supply_from_transfers)

    def update_from_transaction(self, transaction: Dict) -> None:
        """Ingest a processed transaction and update all transfer/approval state."""

        self.add_transfers(transaction)
        self.add_internal_eth_transfers(transaction)
        self.add_approvals(transaction)
        self.update_address_tx_counter(transaction.get("unique_addresses") or [])

    def add_transfers(self, transaction: Dict) -> None:
        """Process transfer events for all relevant tokens."""

        for transfer in transaction.get("erc20_transfers", []):
            try:
                token_address = transfer["token_address"]

                if token_address == self.contract_address:
                    self._add_erc20_transfer(transaction, transfer)
                    continue

                pool = self.pool_manager.get_pool(token_address) if self.pool_manager else None
                if pool and pool.get_protocol() == UNISWAP_V2_PROTOCOL:
                    transfer["block_number"] = transaction["block_number"]
                    transfer["tx_hash"] = transaction["hash"]
                    pool.process_lp_transfer(transfer)
                    continue

                if token_address == DENOM_NAMES_TO_ADDRESS["WETH"]:
                    self._add_weth_transfer(transaction, transfer)
                elif token_address in DENOM_NAMES_TO_ADDRESS.values():
                    self._add_other_token_transfer(transaction, transfer)
            except Exception as exc:  # noqa: BLE001
                raise RuntimeError(
                    "TokenTransferTracker.add_transfers failed: "
                    f"token={transfer.get('token_address')} "
                    f"tx={transaction.get('hash')} error={exc}"
                ) from exc

    def add_internal_eth_transfers(self, transaction: Dict) -> None:
        """Record internal ETH transfers for the transaction."""

        tx_hash = transaction["hash"]
        block_number = transaction["block_number"]
        tx_index = transaction["tx_index"]
        for transfer in transaction.get("internal_transactions", []):
            transfer_dict = {
                "tx_hash": tx_hash,
                "block_number": block_number,
                "tx_index": tx_index,
                "depth": transfer["depth"],
                "from_address": transfer["from_address"],
                "to_address": transfer["to_address"],
                "amount": float(transfer["value"]),
                "token_address": "ETH",
            }
            bounded_history.append_to_dict_history(
                self.eth_transfers, tx_hash, transfer_dict, self.history_limit
            )

    def add_approvals(self, transaction: Dict) -> None:
        """Process approval events."""

        tx_hash = transaction["hash"]
        block_number = transaction["block_number"]
        tx_index = transaction["tx_index"]
        for approval in transaction.get("approvals", []):
            token_address = approval["token_address"]

            pool = self.pool_manager.get_pool(token_address) if self.pool_manager else None
            if pool and pool.get_protocol() == UNISWAP_V2_PROTOCOL:
                approval["block_number"] = block_number
                approval["tx_hash"] = tx_hash
                approval["block_timestamp"] = transaction.get("block_timestamp")
                pool.process_lp_approval(approval)

            bounded_history.append_with_history_limit(
                self.approvals,
                {
                    "tx_hash": tx_hash,
                    "block_number": block_number,
                    "tx_index": tx_index,
                    "log_index": approval["log_index"],
                    "owner": approval["owner"],
                    "spender": approval.get("spender") or approval.get("approved_address"),
                    "token_address": approval["token_address"],
                },
                self.history_limit,
            )
            spender = approval.get("spender") or approval.get("approved_address")
            if spender:
                self.approved_addresses.add(spender)

    def update_address_tx_counter(self, unique_addresses: Iterable[str]) -> None:
        """Update the transaction counter for every unique address seen."""

        for unique_address in unique_addresses:
            if unique_address not in self.address_tx_counter:
                self.address_tx_counter[unique_address] = 0
            self.address_tx_counter[unique_address] += 1
    
    def _add_erc20_transfer(self, transaction: Dict, transfer: Dict) -> None:
        tx_hash = transaction["hash"]
        block_number = transaction["block_number"]
        tx_index = transaction["tx_index"]
        amount = int(transfer["amount"]) / 10 ** self.decimals
        transfer_dict = {
            "tx_hash": tx_hash,
            "block_number": block_number,
            "tx_index": tx_index,
            "log_index": transfer["log_index"],
            "from_address": transfer["from_address"],
            "to_address": transfer["to_address"],
            "amount": amount,
            "token_address": transfer["token_address"],
        }
        bounded_history.append_to_dict_history(
            self.erc20_transfers, tx_hash, transfer_dict, self.history_limit
        )
        if transfer["from_address"] == "0x0000000000000000000000000000000000000000":
            self.total_supply_from_transfers += amount

    def _add_weth_transfer(self, transaction: Dict, transfer: Dict) -> None:
        tx_hash = transaction["hash"]
        block_number = transaction["block_number"]
        tx_index = transaction["tx_index"]
        transfer_dict = {
            "tx_hash": tx_hash,
            "block_number": block_number,
            "tx_index": tx_index,
            "log_index": transfer["log_index"],
            "from_address": transfer["from_address"],
            "to_address": transfer["to_address"],
            "amount": float(transfer["amount"]) / 10**18,
            "token_address": "WETH",
        }
        bounded_history.append_to_dict_history(
            self.eth_transfers, tx_hash, transfer_dict, self.history_limit
        )

    def _add_other_token_transfer(self, transaction: Dict, transfer: Dict) -> None:
        tx_hash = transaction["hash"]
        block_number = transaction["block_number"]
        tx_index = transaction["tx_index"]
        if transfer["token_address"] in DENOM_ADDRESSES:
            denom_name = DENOM_ADDRESSES[transfer["token_address"]]
            denom_decimals = ERC20_TOKEN_DECIMALS[denom_name]
            amount = float(transfer["amount"]) / 10**denom_decimals
            if denom_name not in self.other_currencies:
                self.other_currencies[denom_name] = 0
            self.other_currencies[denom_name] += 1

            transfer_dict = {
                "tx_hash": tx_hash,
                "block_number": block_number,
                "tx_index": tx_index,
                "log_index": transfer["log_index"],
                "from_address": transfer["from_address"],
                "to_address": transfer["to_address"],
                "amount": amount,
                "token_address": transfer["token_address"],
            }
            bounded_history.append_to_dict_history(
                self.other_denom_transfers, tx_hash, transfer_dict, self.history_limit
            )

    def set_pool_manager(self, pool_manager) -> None:
        self.pool_manager = pool_manager
