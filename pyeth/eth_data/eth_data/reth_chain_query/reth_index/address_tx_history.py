"""Reth-backed helpers for retrieving address transaction history."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable, List, Optional, Sequence
from web3 import Web3
from pyreth import chain_query


@dataclass(frozen=True)
class AddressTxRecord:
    """Represents one transaction the address participated in."""

    block_number: int
    tx_index: int
    tx_number: int
    tx_hash: str

    @classmethod
    def from_pyreth(cls, ref) -> "AddressTxRecord":  # pragma: no cover
        return cls(
            block_number=int(ref.block_number),
            tx_index=int(ref.tx_index),
            tx_number=int(ref.tx_number),
            tx_hash=str(ref.tx_hash),
        )


class RethAddressTxHistory:
    """High-level access to the Reth-backed address→tx history."""

    def __init__(self, pyreth_client: Optional[Any] = None) -> None:
        self._chain_query = pyreth_client.chain_query() if pyreth_client is not None else chain_query()

    @staticmethod
    def _normalise_address(address: str) -> str:
        return Web3.to_checksum_address(address)

    def get_transactions(
        self,
        address: str,
        *,
        limit: Optional[int] = None,
        reverse: bool = False,
    ) -> List[AddressTxRecord]:
        """Return transactions involving ``address``.

        Args:
            address: Address with or without 0x prefix.
            limit: Optional cap on number of returned records (applied after ordering).
            reverse: If True return newest-first, otherwise oldest-first.
        """

        normalised = self._normalise_address(address)
        refs = self._chain_query.address_transactions(normalised)
        records: Sequence[AddressTxRecord] = [
            AddressTxRecord.from_pyreth(ref) for ref in refs
        ]
        if reverse:
            records = list(reversed(records))
        if limit is not None:
            records = list(records[:limit])
        return list(records)

    def newest_transactions(
        self, address: str, *, limit: Optional[int] = None
    ) -> List[AddressTxRecord]:
        """Convenience helper returning newest-first transactions."""

        return self.get_transactions(address, limit=limit, reverse=True)

    def has_block_indices(self, block_number: int) -> bool:
        """Whether the address index currently exposes ``block_number``."""

        return bool(self._chain_query.block_has_indices(int(block_number)))

    def iter_transactions(self, address: str) -> Iterable[AddressTxRecord]:
        """Yield transactions in ascending block order for ``address``."""

        for record in self.get_transactions(address):
            yield record
