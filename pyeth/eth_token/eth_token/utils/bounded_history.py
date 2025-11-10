"""Helpers for history-bounded append operations."""

from typing import Hashable, MutableMapping, List, MutableSequence

__all__ = [
    "append_with_history_limit",
    "append_to_dict_history",
]


def append_with_history_limit(items: MutableSequence, entry, limit: int) -> None:
    """Append ``entry`` to ``items`` and trim the prefix to ``limit`` entries."""

    items.append(entry)
    if len(items) > limit:
        del items[: len(items) - limit]


def append_to_dict_history(
    mapping: MutableMapping[Hashable, List],
    key: Hashable,
    entry,
    limit: int,
) -> None:
    """Append ``entry`` under ``key`` and trim the per-key and global bounds."""

    bucket = mapping.setdefault(key, [])
    bucket.append(entry)
    if len(bucket) > limit:
        del bucket[: len(bucket) - limit]
    while len(mapping) > limit:
        oldest_key = next(iter(mapping))
        if oldest_key == key and len(mapping) == 1:
            break
        mapping.pop(oldest_key)
