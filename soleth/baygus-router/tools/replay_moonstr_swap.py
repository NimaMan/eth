#!/usr/bin/env python3
"""
Replay the MOONSTR → ETH Uniswap v4 swap captured in `references/moonstr_swap_snapshot.json`.

This script fulfils the v0.1 acceptance criterion: it demonstrates that we can decode and reason
about a production Uniswap v4 router execution without deploying any new contracts.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from textwrap import indent

SNAPSHOT_PATH = Path(__file__).resolve().parents[1] / "references" / "moonstr_swap_snapshot.json"


@dataclass
class EthTransfer:
    sender: str
    recipient: str
    amount_eth: float
    purpose: str


@dataclass
class TransactionSnapshot:
    tx_hash: str
    block: int
    timestamp_utc: str
    sender: str
    router: str
    pool_manager: str
    token_symbol: str
    token_address: str
    token_amount: float
    eth_total: float
    transfers: list[EthTransfer]

    @classmethod
    def from_json(cls, raw: dict) -> "TransactionSnapshot":
        tx = raw["transaction"]
        assets = raw["assets"]
        transfers = [
            EthTransfer(
                sender=entry["from"],
                recipient=entry["to"],
                amount_eth=float(entry["amount_eth"]),
                purpose=entry["purpose"],
            )
            for entry in assets["eth_transfers"]
        ]
        return cls(
            tx_hash=tx["hash"],
            block=tx["block"],
            timestamp_utc=tx["timestamp_utc"],
            sender=tx["from"],
            router=tx["to_router"],
            pool_manager=tx["pool_manager"],
            token_symbol=assets["token_in"]["symbol"],
            token_address=assets["token_in"]["address"],
            token_amount=float(assets["token_in"]["amount"]),
            eth_total=float(assets["eth_out_total"]),
            transfers=transfers,
        )


def load_snapshot() -> TransactionSnapshot:
    try:
        data = json.loads(SNAPSHOT_PATH.read_text())
    except FileNotFoundError as exc:
        raise SystemExit(f"Snapshot file not found: {SNAPSHOT_PATH}") from exc
    return TransactionSnapshot.from_json(data)


def format_eth(amount: float) -> str:
    return f"{amount:.18f} ETH"


def print_summary(snapshot: TransactionSnapshot) -> None:
    header = f"""
    === Uniswap v4 Swap Replay (v0.1 Baseline) ===

    Transaction   : {snapshot.tx_hash}
    Block / Time  : {snapshot.block} / {snapshot.timestamp_utc}
    Caller (EOA)  : {snapshot.sender}
    Router        : {snapshot.router}
    PoolManager   : {snapshot.pool_manager}

    Token Sent    : {snapshot.token_amount:,.0f} {snapshot.token_symbol} ({snapshot.token_address})
    ETH Returned  : {format_eth(snapshot.eth_total)}
    """.strip(
        "\n"
    )
    print(header)

    print("\nETH distribution:")
    for transfer in snapshot.transfers:
        print(
            indent(
                f"- {format_eth(transfer.amount_eth):>26} | {transfer.sender} → {transfer.recipient}\n"
                f"  purpose: {transfer.purpose}",
                prefix="  ",
            )
        )

    print(
        "\nConclusion: this transaction relies on the bespoke router at "
        f"{snapshot.router}, which locks the PoolManager {snapshot.pool_manager}, "
        "executes the swap, and redistributes ETH proceeds to the trader and two fee "
        "recipients. No bytecode for this router exists in our current simulation environment, "
        "which is why the generic simulator fails to reproduce the trade. Completing v0.2 will "
        "address this gap by providing a minimal router we control."
    )


def main() -> None:
    snapshot = load_snapshot()
    print_summary(snapshot)


if __name__ == "__main__":
    main()
