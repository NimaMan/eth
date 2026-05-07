from collections import defaultdict
from typing import Dict, List, Optional, Sequence, Set, Tuple

import pyreth
from pyreth import block_processor


def main() -> None:
    provider = block_processor()
    latest_block = provider.get_latest_block()
    processed_block = provider.process_block(latest_block)

    entries = _entries_from_address_index(processed_block)
    if entries is None:
        entries = _entries_from_transactions(processed_block.transactions)

    indexer = pyreth.AddressBlockParticipationIndexer()
    inserted = indexer.write_block_participation(latest_block, entries)
    print(
        f"Indexed block {latest_block} with {len(entries)} participating txs; "
        f"appended {inserted} address-to-block entries",
    )


def _entries_from_address_index(processed_block) -> Optional[List[Tuple[int, List[str]]]]:
    address_index = getattr(processed_block, "address_index", None)
    if not address_index:
        return None

    tx_to_addresses: Dict[int, Set[str]] = defaultdict(set)
    for address, transactions in address_index.items():
        if not address or not transactions:
            continue
        for tx in transactions:
            tx_index = _get_tx_index(tx)
            if tx_index is None:
                continue
            tx_to_addresses[tx_index].add(address)

    if not tx_to_addresses:
        return None

    return [
        (tx_index, sorted(addresses))
        for tx_index, addresses in sorted(tx_to_addresses.items())
        if addresses
    ]


def _entries_from_transactions(transactions: Sequence[object]) -> List[Tuple[int, List[str]]]:
    entries: List[Tuple[int, List[str]]] = []
    for tx in transactions:
        addresses = set(getattr(tx, "unique_addresses", []) or [])
        from_address = getattr(tx, "from_address", None)
        to_address = getattr(tx, "to_address", None)
        if not addresses and isinstance(tx, dict):
            raw = tx.get("unique_addresses")
            if raw:
                addresses.update(raw)
            from_address = from_address or tx.get("from_address")
            to_address = to_address or tx.get("to_address")
        if from_address:
            addresses.add(from_address)
        if to_address:
            addresses.add(to_address)
        if not addresses:
            continue
        tx_index = _get_tx_index(tx)
        if tx_index is None:
            continue
        entries.append((tx_index, sorted(addresses)))
    return entries


def _get_tx_index(tx: object) -> Optional[int]:
    tx_index = getattr(tx, "tx_index", None)
    if tx_index is None and isinstance(tx, dict):
        tx_index = tx.get("tx_index")
    if tx_index is None:
        return None
    try:
        return int(tx_index)
    except (TypeError, ValueError):
        return None


if __name__ == "__main__":
    main()
