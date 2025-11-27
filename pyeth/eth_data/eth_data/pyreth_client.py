"""Centralized access to shared PyReth resources.

Usage:
    client = PyrethClient.instance()
    chain_query = client.chain_query()
    processed_provider = client.processed_tx_provider()
"""

from __future__ import annotations

import threading
from typing import Optional

import pyreth


class PyrethClient:
    """Singleton wrapper that shares a single PyReth handle across callers."""

    _instance: Optional["PyrethClient"] = None
    _lock = threading.Lock()

    def __init__(self) -> None:
        self._pyreth = pyreth.PyReth()
        self._chain_query = None
        self._processed_tx_provider = None
        self._price_client = None
        self._pool_buy_sell_simulator = None
        self._address_indexers = {}

    @classmethod
    def instance(cls) -> "PyrethClient":
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    @property
    def pyreth(self) -> pyreth.PyReth:
        return self._pyreth

    def chain_query(self):
        if self._chain_query is None:
            self._chain_query = self._pyreth.chain_query()
        return self._chain_query

    def tx_processor(self):
        return self._pyreth.tx_processor()

    def processed_tx_provider(self):
        if self._processed_tx_provider is None:
            self._processed_tx_provider = self._pyreth.processed_tx_provider()
        return self._processed_tx_provider

    def price_client(self):
        if self._price_client is None:
            self._price_client = self._pyreth.price_client()
        return self._price_client

    def pool_buy_sell_simulator(self):
        if self._pool_buy_sell_simulator is None:
            self._pool_buy_sell_simulator = self._pyreth.pool_buy_sell_simulator()
        return self._pool_buy_sell_simulator

    def address_indexer(self, *, read_only: bool = False):
        """Return the address index handle."""
        key = bool(read_only)
        if key not in self._address_indexers:
            if key:
                self._address_indexers[key] = pyreth.AddressTxIndexFetcher()
            else:
                self._address_indexers[key] = pyreth.AddressTxIndexer()
        return self._address_indexers[key]

    def clear_cached_handles(self) -> None:
        """Drop cached handles so subsequent calls rebuild them."""
        self._chain_query = None
        self._processed_tx_provider = None
        self._price_client = None
        self._pool_buy_sell_simulator = None
        self._address_indexers = {}

    @classmethod
    def reset(cls) -> None:
        """Reset the singleton (mainly for tests)."""
        with cls._lock:
            if cls._instance is not None:
                cls._instance.clear_cached_handles()
            cls._instance = None
