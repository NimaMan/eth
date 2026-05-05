#!/usr/bin/env python3
"""
Analyse the most recent processed transactions for jaredfromsubway and
summarise the net cash flows for the key addresses involved.

Usage:
    python latest_tx_profit.py [--limit 100] [--show-details]
"""

from __future__ import annotations

import argparse
from collections import defaultdict, deque
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal, getcontext
from typing import Any, Dict, Iterable, List, Optional

import pyreth
from pyreth import block_processor, chain_query as pyreth_chain_query
from eth_token.erc20_token.pools.addresses import require_checksum_address, same_address
from eth_data.reth_chain_query.reth_index.address_tx_history import RethAddressTxHistory

getcontext().prec = 40


WATCHED_ENTITIES: Dict[str, str] = {
    "jared_eoa": "0xae2Fc483527B8EF99EB5D9B44875F005ba1FaE13",
    "bundle_contract": "0x1f2F10D1C40777AE1Da742455c65828FF36Df387",
    "payout_wallet": "0x02c552AFB2F5C7b8e8253e5a28Dcdf2AD68Cdb3D",
}
WATCHED_ENTITIES = {
    alias: require_checksum_address(address)
    for alias, address in WATCHED_ENTITIES.items()
}

ADDRESS_TO_ALIAS = {addr: alias for alias, addr in WATCHED_ENTITIES.items()}
WETH_ADDRESS = require_checksum_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")


@dataclass
class TokenAmount:
    token_address: str
    symbol: str
    amount: Decimal
    raw_amount: int
    decimals: int


@dataclass
class AddressFlow:
    address: str
    alias: str
    currency: Dict[str, Decimal] = field(default_factory=dict)
    tokens: Dict[str, TokenAmount] = field(default_factory=dict)

    def add_currency(self, symbol: str, amount: Decimal) -> None:
        self.currency[symbol] = self.currency.get(symbol, Decimal(0)) + amount

    def add_token(
        self,
        token_address: str,
        symbol: str,
        amount: Decimal,
        raw_amount: int,
        decimals: int,
    ) -> None:
        existing = self.tokens.get(token_address)
        if existing:
            existing.amount += amount
            existing.raw_amount += raw_amount
        else:
            self.tokens[token_address] = TokenAmount(
                token_address=token_address,
                symbol=symbol,
                amount=amount,
                raw_amount=raw_amount,
                decimals=decimals,
            )


@dataclass
class TransactionSummary:
    tx_hash: str
    block_number: int
    tx_index: int
    timestamp: datetime
    from_address: str
    to_address: Optional[str]
    gas_paid_eth: Decimal
    flows: Dict[str, AddressFlow]


@dataclass
class AggregateFlow:
    alias: str
    eth: Decimal = Decimal(0)
    weth: Decimal = Decimal(0)
    other_tokens: Dict[str, Dict[str, Decimal]] = field(default_factory=dict)

    def add_token(self, token: TokenAmount) -> None:
        entry = self.other_tokens.setdefault(token.token_address, {"symbol": token.symbol, "amount": Decimal(0)})
        entry["amount"] += token.amount


class TokenMetadataResolver:
    def __init__(self, chain_query: pyreth.ChainQuery):
        self._chain_query = chain_query
        self._cache: Dict[str, Dict[str, object]] = {}

    def get(self, token_address: str, block_number: int) -> Dict[str, object]:
        key = require_checksum_address(token_address)
        if key not in self._cache:
            decimals = 18
            symbol = token_address[:6]
            try:
                decimals = self._chain_query.get_token_decimals(token_address, block_number)
            except Exception:
                pass
            try:
                fetched_symbol = self._chain_query.get_token_symbol(token_address, block_number)
                if fetched_symbol:
                    symbol = str(fetched_symbol)
            except Exception:
                pass
            self._cache[key] = {"decimals": decimals, "symbol": symbol}
        return self._cache[key]


class LatestTransactionFetcher:
    def __init__(
        self,
        provider: Any,
        history: RethAddressTxHistory,
        chain_query: pyreth.ChainQuery,
    ):
        self._provider = provider
        self._history = history
        self._chain_query = chain_query
        self._address_provider = provider.address_provider()

    def _load_processed(self, records: Iterable["AddressTxRecord"]) -> List["ProcessedTransaction"]:
        processed: List["ProcessedTransaction"] = []
        for record in records:
            try:
                tx = self._provider.processed_transaction_by_hash(record.tx_hash)
            except Exception as exc:
                print(f"warning: could not load transaction {record.tx_hash}: {exc}")
                continue
            processed.append(tx)
        processed.sort(key=lambda tx: (tx.block_number, tx.tx_index))
        return processed

    def fetch(self, address: str, limit: int) -> List["ProcessedTransaction"]:
        records = self._history.newest_transactions(address, limit=limit)
        return self._load_processed(records)

    def fetch_since(
        self,
        address: str,
        since_timestamp: datetime,
        initial_limit: int = 200,
        max_limit: int = 5000,
    ) -> List["ProcessedTransaction"]:
        limit = initial_limit
        since_ts = int(since_timestamp.timestamp())
        processed: List["ProcessedTransaction"] = []

        while True:
            try:
                records = self._history.newest_transactions(address, limit=limit)
            except RuntimeError as exc:
                print(
                    f"warning: newest_transactions failed ({exc}), falling back to block scan"
                )
                return self._fallback_block_scan(address, since_timestamp)
            processed = self._load_processed(records)
            if not processed:
                return []
            oldest_ts = processed[0].block_timestamp
            if oldest_ts <= since_ts or len(records) < limit or limit >= max_limit:
                break
            if limit >= max_limit:
                break
            limit = min(limit * 2, max_limit)

        recent = [tx for tx in processed if tx.block_timestamp >= since_ts]
        return recent

    def _fallback_block_scan(self, address: str, since_timestamp: datetime) -> List["ProcessedTransaction"]:
        since_ts = int(since_timestamp.timestamp())
        try:
            latest_block = self._chain_query.get_latest_block()
            since_block = self._chain_query.timestamp_to_block(since_timestamp.isoformat())
        except Exception as exc:
            print(f"warning: failed to resolve block range for fallback scan: {exc}")
            return []

        chunk_start = since_block
        chunk_size = 3000

        while chunk_start <= latest_block:
            chunk_end = min(chunk_start + chunk_size, latest_block)
            current_end = chunk_end
            loaded = False
            while current_end >= chunk_start:
                try:
                    self._address_provider.load_blocks_for_address(address, chunk_start, current_end)
                    loaded = True
                    break
                except Exception as exc:
                    message = str(exc)
                    if "No header for block" in message and current_end > chunk_start:
                        current_end -= 1
                        continue
                    print(f"warning: fallback block scan failed to load blocks: {exc}")
                    loaded = False
                    break
            if not loaded:
                chunk_start = current_end + 1
                continue

            chunk_start = current_end + 1

        try:
            transactions = self._address_provider.transactions_for(address)
        except Exception as exc:
            print(f"warning: fallback block scan failed to retrieve transactions: {exc}")
            return []

        result = [
            tx for tx in transactions if getattr(tx, "block_timestamp", 0) >= since_ts
        ]
        result.sort(key=lambda tx: (tx.block_number, tx.tx_index))
        return result


class ProfitCalculator:
    def __init__(self, chain_query: pyreth.ChainQuery):
        self._metadata = TokenMetadataResolver(chain_query)

    def _eth_delta_from_change(self, change: dict, block_number: int) -> Decimal:
        delta = Decimal(0)
        currency_net = change.get("currency_net") or {}
        for currency, amount in currency_net.items():
            if currency.upper() == "ETH":
                delta += Decimal(str(amount))

        token_net = change.get("token_net") or {}
        for token_address, raw_amount in token_net.items():
            if not same_address(token_address, WETH_ADDRESS):
                continue
            metadata = self._metadata.get(token_address, block_number)
            decimals = int(metadata["decimals"])
            delta += Decimal(str(raw_amount)) / (Decimal(10) ** decimals)
        return delta

    def _gas_fee_delta(self, tx: "ProcessedTransaction") -> Decimal:
        fees = tx.fees or {}
        if not fees:
            return Decimal(0)
        return Decimal(int(fees.get("tx_fee", "0"))) / Decimal(10**18)

    def summarise(self, tx: "ProcessedTransaction") -> TransactionSummary:
        flows: Dict[str, AddressFlow] = {}
        balance_changes = tx.address_balance_changes or {}

        for address, change in balance_changes.items():
            alias = ADDRESS_TO_ALIAS.get(require_checksum_address(address))
            if not alias:
                continue
            flow = flows.setdefault(alias, AddressFlow(address=address, alias=alias))
            for currency, amount in (change.get("currency_net") or {}).items():
                flow.add_currency(currency, Decimal(str(amount)))
            for token_address, raw_amount_str in (change.get("token_net") or {}).items():
                metadata = self._metadata.get(token_address, tx.block_number)
                raw_amount = int(Decimal(str(raw_amount_str)))
                decimals = int(metadata["decimals"])
                amount = Decimal(raw_amount) / (Decimal(10) ** decimals)
                flow.add_token(
                    token_address=token_address,
                    symbol=str(metadata["symbol"]),
                    amount=amount,
                    raw_amount=raw_amount,
                    decimals=decimals,
                )

        gas_paid_eth = Decimal(0)
        fees = tx.fees or {}
        if fees:
            gas_paid_eth = self._gas_fee_delta(tx)
            alias = ADDRESS_TO_ALIAS.get(require_checksum_address(tx.from_address))
            if alias:
                flow = flows.setdefault(
                    alias,
                    AddressFlow(address=tx.from_address, alias=alias),
                )
                flow.add_currency("ETH", -gas_paid_eth)

        timestamp = datetime.fromtimestamp(tx.block_timestamp, tz=timezone.utc)

        return TransactionSummary(
            tx_hash=tx.hash,
            block_number=tx.block_number,
            tx_index=tx.tx_index,
            timestamp=timestamp,
            from_address=tx.from_address,
            to_address=getattr(tx, "to_address", None),
            gas_paid_eth=gas_paid_eth,
            flows=flows,
        )

    def aggregate(self, summaries: Iterable[TransactionSummary]) -> Dict[str, AggregateFlow]:
        totals = {alias: AggregateFlow(alias=alias) for alias in WATCHED_ENTITIES.keys()}
        for summary in summaries:
            for alias, flow in summary.flows.items():
                aggregate = totals.setdefault(alias, AggregateFlow(alias=alias))
                eth_delta = flow.currency.get("ETH", Decimal(0))
                aggregate.eth += eth_delta
                for token_amount in flow.tokens.values():
                    if same_address(token_amount.token_address, WETH_ADDRESS):
                        aggregate.weth += token_amount.amount
                    else:
                        aggregate.add_token(token_amount)
        return totals

    def eth_deltas(self, tx: "ProcessedTransaction") -> Dict[str, Decimal]:
        deltas: Dict[str, Decimal] = {}
        balance_changes = tx.address_balance_changes or {}
        for address, change in balance_changes.items():
            delta = self._eth_delta_from_change(change, tx.block_number)
            if delta:
                deltas[address] = deltas.get(address, Decimal(0)) + delta

        gas_delta = self._gas_fee_delta(tx)
        if gas_delta:
            deltas[tx.from_address] = deltas.get(tx.from_address, Decimal(0)) - gas_delta
        return deltas

    def aggregate_network(self, transactions: Iterable["ProcessedTransaction"]) -> Dict[str, Decimal]:
        network: Dict[str, Decimal] = defaultdict(Decimal)
        for tx in transactions:
            for address, delta in self.eth_deltas(tx).items():
                if delta:
                    network[address] += delta
        return network


class ProfitReporter:
    def __init__(self, show_details: bool):
        self._show_details = show_details

    def _format_decimal(self, value: Decimal, precision: int = 6) -> str:
        if value == 0:
            return "0"
        format_str = f"{{:+.{precision}f}}"
        return format_str.format(value)

    def _format_token_amount(self, token: TokenAmount) -> str:
        precision = 6 if token.decimals >= 6 else token.decimals
        return f"{token.symbol}:{self._format_decimal(token.amount, precision)}"

    def _format_other_tokens(self, aggregate: AggregateFlow, limit: int = 10) -> str:
        if not aggregate.other_tokens:
            return "-"
        sorted_tokens = sorted(
            aggregate.other_tokens.items(),
            key=lambda item: abs(item[1]["amount"]),
            reverse=True,
        )
        parts = []
        for (token_address, data) in sorted_tokens[:limit]:
            symbol = data["symbol"]
            amount = data["amount"]
            parts.append(f"{symbol}:{self._format_decimal(amount)}")
        if len(sorted_tokens) > limit:
            parts.append("…")
        return ", ".join(parts)

    def print_details(self, summaries: Iterable[TransactionSummary]) -> None:
        for summary in summaries:
            print(
                f"[{summary.block_number}:{summary.tx_index}] "
                f"{summary.timestamp.isoformat()} "
                f"{summary.tx_hash}"
            )
            for alias, flow in summary.flows.items():
                eth_delta = flow.currency.get("ETH", Decimal(0))
                token_parts = [self._format_token_amount(token) for token in flow.tokens.values()]
                token_display = ", ".join(token_parts) if token_parts else "-"
                print(
                    f"  {alias:<16} ETH {self._format_decimal(eth_delta)} | tokens {token_display}"
                )
            if not summary.flows:
                print("  (no watched address deltas)")
            if summary.gas_paid_eth:
                print(f"  gas paid: {self._format_decimal(summary.gas_paid_eth)} ETH by {summary.from_address}")
            print("")

    def print_totals(self, totals: Dict[str, AggregateFlow]) -> None:
        print("Aggregate net flows:")
        for alias, aggregate in totals.items():
            eth_part = self._format_decimal(aggregate.eth)
            weth_part = self._format_decimal(aggregate.weth)
            other_display = self._format_other_tokens(aggregate)
            print(
                f"  {alias:<16} ETH {eth_part} | WETH {weth_part} | other {other_display}"
            )

        combined = sum((flow.eth + flow.weth) for flow in totals.values())
        print(f"\nCombined ETH (ETH + WETH across tracked addresses): {self._format_decimal(combined)}")

    def print_network(self, network: Dict[str, Decimal], top_n: int = 10) -> None:
        if not network:
            print("No ETH/WETH deltas captured for the selected window.")
            return
        sorted_addrs = sorted(network.items(), key=lambda item: item[1], reverse=True)
        positives = [(addr, delta) for addr, delta in sorted_addrs if delta > 0]
        negatives = [(addr, delta) for addr, delta in sorted_addrs if delta < 0]

        def describe(entries):
            lines = []
            for address, delta in entries[:top_n]:
                alias = ADDRESS_TO_ALIAS.get(require_checksum_address(address), "")
                tag = f" ({alias})" if alias else ""
                lines.append(f"  {address}{tag}: {self._format_decimal(delta)}")
            return "\n".join(lines) if lines else "  (none)"

        print("\nTop positive ETH recipients:")
        print(describe(positives))
        print("\nTop negative ETH spenders:")
        print(describe(list(reversed(negatives))))

    def print_edges(self, edges: List["FlowEdge"], top_n: int = 10) -> None:
        if not edges:
            print("No edges captured for the selected window.")
            return
        sorted_edges = sorted(edges, key=lambda edge: edge.amount, reverse=True)
        print("\nTop edges by absolute ETH flow:")
        for edge in sorted_edges[:top_n]:
            src_alias = ADDRESS_TO_ALIAS.get(require_checksum_address(edge.source), "")
            dst_alias = ADDRESS_TO_ALIAS.get(require_checksum_address(edge.target), "")
            src_tag = f" ({src_alias})" if src_alias else ""
            dst_tag = f" ({dst_alias})" if dst_alias else ""
            print(
                f"  {edge.source}{src_tag} -> {edge.target}{dst_tag}: "
                f"{self._format_decimal(edge.amount)} (tx {edge.tx_hash})"
            )


@dataclass
class FlowEdge:
    source: str
    target: str
    amount: Decimal
    tx_hash: str


@dataclass
class FlowNetwork:
    node_values: Dict[str, Decimal] = field(default_factory=dict)
    edges: List[FlowEdge] = field(default_factory=list)


class NetworkBuilder:
    def __init__(self, calculator: ProfitCalculator):
        self._calculator = calculator

    @staticmethod
    def _normalize(deltas: Dict[str, Decimal]) -> Dict[str, Decimal]:
        normalized = {}
        for address, delta in deltas.items():
            if delta == 0:
                continue
            normalized[address] = delta
        return normalized

    def _build_edges_for_tx(self, tx: "ProcessedTransaction") -> List[FlowEdge]:
        deltas = self._calculator.eth_deltas(tx)
        deltas = self._normalize(deltas)
        positives = [(addr, delta) for addr, delta in deltas.items() if delta > 0]
        negatives = [(addr, -delta) for addr, delta in deltas.items() if delta < 0]
        if not positives or not negatives:
            return []

        pos_queue = deque((addr, amount) for addr, amount in positives)
        neg_queue = deque((addr, amount) for addr, amount in negatives)
        edges: List[FlowEdge] = []

        while neg_queue and pos_queue:
            src_addr, src_amount = neg_queue[0]
            dst_addr, dst_amount = pos_queue[0]
            flow = min(src_amount, dst_amount)
            if flow > 0:
                edges.append(
                    FlowEdge(
                        source=src_addr,
                        target=dst_addr,
                        amount=flow,
                        tx_hash=tx.hash,
                    )
                )
            src_amount -= flow
            dst_amount -= flow
            neg_queue[0] = (src_addr, src_amount)
            pos_queue[0] = (dst_addr, dst_amount)
            if src_amount <= Decimal("1e-18"):
                neg_queue.popleft()
            else:
                neg_queue[0] = (src_addr, src_amount)
            if dst_amount <= Decimal("1e-18"):
                pos_queue.popleft()
            else:
                pos_queue[0] = (dst_addr, dst_amount)

        return edges

    def build(self, transactions: Iterable["ProcessedTransaction"]) -> FlowNetwork:
        node_values: Dict[str, Decimal] = defaultdict(Decimal)
        edges: List[FlowEdge] = []

        for tx in transactions:
            edges.extend(self._build_edges_for_tx(tx))
            deltas = self._calculator.eth_deltas(tx)
            for address, delta in deltas.items():
                node_values[address] += delta

        return FlowNetwork(node_values=dict(node_values), edges=edges)


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Summarise the latest jaredfromsubway transactions.")
    parser.add_argument("--limit", type=int, default=100, help="Number of latest transactions to analyse.")
    parser.add_argument(
        "--since-hours",
        type=float,
        default=None,
        help="If provided, analyse transactions from the last N hours instead of a fixed count.",
    )
    parser.add_argument(
        "--show-details",
        action="store_true",
        help="Print per-transaction details in addition to aggregate totals.",
    )
    parser.add_argument(
        "--network-top",
        type=int,
        default=10,
        help="Show the top N addresses by net ETH/WETH delta (positive and negative).",
    )
    parser.add_argument(
        "--edge-top",
        type=int,
        default=10,
        help="Show the top N edges by absolute ETH/WETH flow.",
    )
    return parser


def main() -> None:
    args = build_arg_parser().parse_args()

    provider = block_processor()
    history = RethAddressTxHistory()
    try:
        chain_query = history._chain_query  # pyright: ignore[reportPrivateUsage]
    except AttributeError:
        chain_query = pyreth_chain_query()
    fetcher = LatestTransactionFetcher(provider, history, chain_query)
    calculator = ProfitCalculator(chain_query)
    network_builder = NetworkBuilder(calculator)

    if args.since_hours:
        cutoff = datetime.now(timezone.utc) - timedelta(hours=args.since_hours)
        transactions = fetcher.fetch_since(WATCHED_ENTITIES["jared_eoa"], since_timestamp=cutoff)
    else:
        transactions = fetcher.fetch(WATCHED_ENTITIES["jared_eoa"], limit=args.limit)

    if not transactions:
        print("No transactions found for the requested window.")
        return
    summaries = [calculator.summarise(tx) for tx in transactions]
    totals = calculator.aggregate(summaries)

    reporter = ProfitReporter(show_details=args.show_details)
    if args.show_details:
        reporter.print_details(summaries)
    reporter.print_totals(totals)
    if args.network_top > 0:
        network = calculator.aggregate_network(transactions)
        reporter.print_network(network, top_n=args.network_top)
    if args.edge_top > 0:
        flow_network = network_builder.build(transactions)
        reporter.print_edges(flow_network.edges, top_n=args.edge_top)


if __name__ == "__main__":
    main()
