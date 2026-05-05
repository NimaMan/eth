"""Backward-compatible helpers around the pyreth module-level accessors."""

from typing import Any, Tuple

import pyreth
from pyreth import chain_query


def get_pyreth_chain_query() -> Tuple[Any, Any]:
    """Return the shared PyReth instance and its ChainQuery handle."""
    return pyreth, chain_query()
