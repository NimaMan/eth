#!/usr/bin/env python3
"""
Quick-and-dirty helper for inspecting JaredFromSubway sandwich bundles.

Given any transaction hash from the bundle, the script:
  * identifies every transaction in that block sent from the same searcher EOA
  * fetches each receipt and walks the logs
  * extracts ERC-20 `Transfer` events for WETH so we can see the bot's cash flows
  * decodes Uniswap V2 `Swap` events so we know which pool/token each leg touched
  * prints a compact per-transaction summary including gas spend and net WETH delta

This is not a production-grade trace decoder—it's just enough to answer
“how did this bundle make (or lose) money?” without leaving the CLI.

Usage:
    python sandwich_breakdown.py 0x<tx hash> [--rpc https://...]
"""

from __future__ import annotations

import argparse
import decimal
import functools
from dataclasses import dataclass, field
from typing import Dict, Iterable, List, Tuple

from eth_abi import decode
from web3 import Web3
from web3.contract import Contract
from web3.types import EventData, LogReceipt

WETH = Web3.to_checksum_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
WETH_TRANSFER = Web3.keccak(text="Transfer(address,address,uint256)").hex()
WETH_WITHDRAW = Web3.keccak(text="Withdrawal(address,uint256)").hex()
UNISWAP_V2_SWAP = Web3.keccak(
    text="Swap(address,uint256,uint256,uint256,uint256,address)"
).hex()

PAIR_ABI = [
    {
        "name": "token0",
        "outputs": [{"type": "address"}],
        "inputs": [],
        "stateMutability": "view",
        "type": "function",
    },
    {
        "name": "token1",
        "outputs": [{"type": "address"}],
        "inputs": [],
        "stateMutability": "view",
        "type": "function",
    },
]

ERC20_ABI = [
    {
        "name": "decimals",
        "outputs": [{"type": "uint8"}],
        "inputs": [],
        "stateMutability": "view",
        "type": "function",
    },
    {
        "name": "symbol",
        "outputs": [{"type": "string"}],
        "inputs": [],
        "stateMutability": "view",
        "type": "function",
    },
]

decimal.getcontext().prec = 40


@dataclass
class WethFlow:
    address: str
    delta_wei: int = 0

    def delta_eth(self) -> decimal.Decimal:
        return decimal.Decimal(self.delta_wei) / decimal.Decimal(10**18)


@dataclass
class SwapLeg:
    pool: str
    token0: str
    token1: str
    amount0_in: int
    amount1_in: int
    amount0_out: int
    amount1_out: int


@dataclass
class TxSummary:
    tx_hash: str
    index: int
    gas_spent_wei: int
    weth_flows: Dict[str, WethFlow] = field(default_factory=dict)
    swap_legs: List[SwapLeg] = field(default_factory=list)

    def add_weth_flow(self, address: str, delta: int) -> None:
        key = address.lower()
        flow = self.weth_flows.setdefault(key, WethFlow(address=address))
        flow.delta_wei += delta

    def pretty_weth(self) -> List[Tuple[str, decimal.Decimal]]:
        return [
            (flow.address, flow.delta_eth())
            for flow in self.weth_flows.values()
            if flow.delta_wei != 0
        ]


@functools.lru_cache(maxsize=256)
def get_pair_contract(w3: Web3, address: str) -> Contract:
    return w3.eth.contract(address=address, abi=PAIR_ABI)


@functools.lru_cache(maxsize=256)
def get_token_meta(w3: Web3, address: str) -> Tuple[str, int]:
    contract = w3.eth.contract(address=address, abi=ERC20_ABI)
    try:
        symbol = contract.functions.symbol().call()
    except Exception:  # pragma: no cover - some tokens revert on symbol()
        symbol = "?"
    try:
        decimals = contract.functions.decimals().call()
    except Exception:  # pragma: no cover - non-standard tokens
        decimals = 18
    return symbol, decimals


def decode_weth_transfers(
    logs: Iterable[LogReceipt], summary: TxSummary, focus_addrs: Iterable[str]
) -> None:
    focus = {addr.lower() for addr in focus_addrs}
    for log in logs:
        if log["address"] != WETH:
            continue
        topic0 = log["topics"][0].hex()
        if topic0 == WETH_TRANSFER:
            from_addr = Web3.to_checksum_address(log["topics"][1].hex()[-40:])
            to_addr = Web3.to_checksum_address(log["topics"][2].hex()[-40:])
            raw = log["data"]
            if isinstance(raw, (bytes, bytearray)):
                value = int.from_bytes(raw, "big")
            else:
                value = int(raw, 16)
            if from_addr.lower() in focus:
                summary.add_weth_flow(from_addr, -value)
            if to_addr.lower() in focus:
                summary.add_weth_flow(to_addr, value)
        elif topic0 == WETH_WITHDRAW:
            to_addr = Web3.to_checksum_address(log["topics"][1].hex()[-40:])
            raw = log["data"]
            if isinstance(raw, (bytes, bytearray)):
                value = int.from_bytes(raw, "big")
            else:
                value = int(raw, 16)
            if to_addr.lower() in focus:
                summary.add_weth_flow(to_addr, -value)


def decode_uniswap_swaps(w3: Web3, logs: Iterable[LogReceipt], summary: TxSummary) -> None:
    for log in logs:
        if not log["topics"]:
            continue
        if log["topics"][0].hex() != UNISWAP_V2_SWAP:
            continue
        pool = Web3.to_checksum_address(log["address"])
        pair = get_pair_contract(w3, pool)
        token0 = pair.functions.token0().call()
        token1 = pair.functions.token1().call()
        raw = log["data"]
        if isinstance(raw, (bytes, bytearray)):
            data_bytes = raw
        else:
            data_bytes = bytes.fromhex(raw[2:])
        amount0_in, amount1_in, amount0_out, amount1_out = decode(
            ["uint256", "uint256", "uint256", "uint256"], data_bytes
        )
        summary.swap_legs.append(
            SwapLeg(
                pool=pool,
                token0=token0,
                token1=token1,
                amount0_in=amount0_in,
                amount1_in=amount1_in,
                amount0_out=amount0_out,
                amount1_out=amount1_out,
            )
        )


def fetch_bundle_transactions(w3: Web3, tx_hash: str) -> Tuple[str, List[str]]:
    tx = w3.eth.get_transaction(tx_hash)
    block = w3.eth.get_block(tx["blockNumber"], full_transactions=True)
    sender = tx["from"].lower()
    bundle_hashes = [
        Web3.to_hex(t["hash"])
        for t in block["transactions"]
        if t["from"].lower() == sender
    ]
    bundle_hashes.sort(
        key=lambda h: w3.eth.get_transaction(h)["transactionIndex"]
    )
    return sender, bundle_hashes


def describe_bundle(w3: Web3, hashes: List[str], focus_addrs: Iterable[str]) -> List[TxSummary]:
    summaries: List[TxSummary] = []
    for h in hashes:
        tx = w3.eth.get_transaction(h)
        receipt = w3.eth.get_transaction_receipt(h)
        gas_spent = receipt["gasUsed"] * receipt["effectiveGasPrice"]
        summary = TxSummary(
            tx_hash=h,
            index=receipt["transactionIndex"],
            gas_spent_wei=gas_spent,
        )
        decode_weth_transfers(receipt["logs"], summary, focus_addrs)
        decode_uniswap_swaps(w3, receipt["logs"], summary)
        summaries.append(summary)
    return summaries


def format_amount(value: decimal.Decimal) -> str:
    if value == 0:
        return "0"
    sign = "+" if value > 0 else "-"
    return f"{sign}{abs(value):.9f}"


def main() -> None:
    parser = argparse.ArgumentParser(description="Inspect JaredFromSubway sandwich flows.")
    parser.add_argument("tx_hash", help="Any transaction hash from the bundle")
    parser.add_argument(
        "--rpc",
        default="https://eth.llamarpc.com",
        help="Ethereum RPC endpoint (default: https://eth.llamarpc.com)",
    )
    parser.add_argument(
        "--watch",
        nargs="*",
        default=[
            "0xae2Fc483527B8EF99EB5D9B44875F005ba1FaE13",
            "0x1f2F10D1C40777AE1Da742455c65828FF36Df387",
            "0x02c552AFB2F5C7b8e8253e5a28Dcdf2AD68Cdb3D",
        ],
        help="Addresses to track for WETH flows (default tracks Jared EOA, Jared bundle contract, and payout EOA)",
    )
    args = parser.parse_args()

    w3 = Web3(Web3.HTTPProvider(args.rpc))
    if not w3.is_connected():
        raise SystemExit(f"Unable to connect to RPC endpoint: {args.rpc}")

    focus_addrs = [Web3.to_checksum_address(addr) for addr in args.watch]
    sender, bundle_hashes = fetch_bundle_transactions(w3, args.tx_hash)

    print(f"Bundle sender: {sender}")
    print("Transactions in bundle (ordered by position in block):")
    for h in bundle_hashes:
        print(f"  - {h}")
    print("")

    summaries = describe_bundle(w3, bundle_hashes, focus_addrs)

    for summary in summaries:
        print(f"tx #{summary.index} :: {summary.tx_hash}")
        print(f"  gas spent   : {decimal.Decimal(summary.gas_spent_wei) / decimal.Decimal(10**18):.9f} ETH")
        weth_lines = summary.pretty_weth()
        if weth_lines:
            print("  WETH flows  :")
            for addr, amount in weth_lines:
                print(f"    {addr} {format_amount(amount)}")
        else:
            print("  WETH flows  : (none)")
        if summary.swap_legs:
            print("  Swaps       :")
            for leg in summary.swap_legs:
                sym0, dec0 = get_token_meta(w3, leg.token0)
                sym1, dec1 = get_token_meta(w3, leg.token1)
                amt0_in = decimal.Decimal(leg.amount0_in) / decimal.Decimal(10**dec0)
                amt1_in = decimal.Decimal(leg.amount1_in) / decimal.Decimal(10**dec1)
                amt0_out = decimal.Decimal(leg.amount0_out) / decimal.Decimal(10**dec0)
                amt1_out = decimal.Decimal(leg.amount1_out) / decimal.Decimal(10**dec1)
                print(
                    f"    pool {leg.pool} :: "
                    f"{sym0} in {amt0_in:.3f} / out {amt0_out:.3f} | "
                    f"{sym1} in {amt1_in:.6f} / out {amt1_out:.6f}"
                )
        else:
            print("  Swaps       : (none)")
        print("")

    net_profit = decimal.Decimal(0)
    for summary in summaries:
        delta = summary.weth_flows.get(focus_addrs[0].lower())
        if delta:
            net_profit += delta.delta_eth()
        delta_contract = summary.weth_flows.get(focus_addrs[1].lower())
        if delta_contract:
            net_profit += delta_contract.delta_eth()

    print(f"Approx net WETH change for tracked addresses: {net_profit:.9f} WETH")


if __name__ == "__main__":
    main()
