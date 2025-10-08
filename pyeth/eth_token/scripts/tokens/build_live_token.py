"""Build a live ERC20 token snapshot using LiveTokenBuilder.

This example mirrors the provider demos by materializing the on-chain state
for the USDC contract (0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48). The builder
pulls processed transactions from PyReth, replays them into the Python token
model, and prints a compact summary.

Run from the repository root after building the PyReth extension:

You can override the token address or block window with CLI flags. Export
PYRETH_DATADIR if your Reth MDBX lives outside the default location.
"""

from __future__ import annotations

import argparse
import asyncio
from time import perf_counter
from typing import Optional, Tuple

from eth_data.chain_utils.common_addresses import DENOM_ADDRESSES
from eth_data.utils import PyrethClient
from eth_token.erc20_token.data.erc20_token_data import TokenStatusEnum
from eth_token.token_builder.live_token_builder import LiveTokenBuilder


TOKEN_ADDRESS = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"  # USDC


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--token",
        default=TOKEN_ADDRESS,
        help="ERC20 token address to build (default: USDC)",
    )
    parser.add_argument(
        "--start-block",
        type=int,
        default=None,
        help="Optional starting block (defaults to builder lookback)",
    )
    parser.add_argument(
        "--end-block",
        type=int,
        default=None,
        help="Optional ending block (defaults to latest)",
    )
    parser.add_argument(
        "--max-blocks",
        type=int,
        default=None,
        help="Maximum number of blocks to scan when start is omitted",
    )
    parser.add_argument(
        "--verify-total-supply",
        action="store_true",
        help="Assert that the rebuilt token total supply matches on-chain metadata (when available).",
    )
    return parser.parse_args()


def fmt_int(value: Optional[int]) -> str:
    if value is None:
        return "-"
    return f"{value:,}"


def resolve_block_bounds(provider, args: argparse.Namespace) -> Tuple[int, int]:
    latest = provider.get_latest_block()
    end_block = int(args.end_block if args.end_block is not None else latest)
    lookback = args.max_blocks

    if args.start_block is None:
        if lookback is None:
            start_block = 0
        else:
            start_block = max(0, end_block + 1 - lookback)
    else:
        start_block = int(args.start_block)
        if args.max_blocks:
            start_block = max(start_block, end_block + 1 - args.max_blocks)

    if start_block > end_block:
        start_block = end_block

    return start_block, end_block


def fetch_metadata(contract_address: str):
    try:
        chain_query = PyrethClient.instance().chain_query()
        return chain_query.get_token_metadata(contract_address, None)
    except Exception:  # noqa: BLE001
        return None


def apply_metadata(token, metadata) -> None:
    if metadata is None:
        return

    data = token.token_data

    if not data.name and getattr(metadata, "name", None):
        data.name = metadata.name
    if not data.symbol and getattr(metadata, "symbol", None):
        data.symbol = metadata.symbol
    if data.decimals is None and getattr(metadata, "decimals", None) is not None:
        data.decimals = int(metadata.decimals)

    total_supply_val = getattr(metadata, "total_supply", None)
    if total_supply_val and (data.total_supply is None or data.total_supply == 0):
        try:
            data.total_supply = int(total_supply_val)
        except ValueError:
            data.total_supply = None

    if not data.token_status:
        data.token_status = TokenStatusEnum.TRADING_ENABLED


def normalize_transaction(tx_dict: dict) -> dict:
    list_keys = [
        "pair_events",
        "uniswap_v2_swaps",
        "uniswap_v2_syncs",
        "uniswap_v3_swaps",
        "uniswap_v3_mints",
        "uniswap_v3_burns",
        "uniswap_v3_decreases",
        "uniswap_v3_increases",
        "uniswap_v3_positions",
        "uniswap_v4_initializes",
        "uniswap_v4_swaps",
        "uniswap_v4_modifies",
    ]

    hex_int_fields = {
        "sqrt_price_x96",
        "reserve0",
        "reserve1",
        "reserve0_raw",
        "reserve1_raw",
        "liquidity",
        "amount0",
        "amount1",
        "amount",
        "price_1e18",
        "amount0In",
        "amount1In",
        "amount0Out",
        "amount1Out",
        "amount0_in",
        "amount1_in",
        "amount0_out",
        "amount1_out",
    }

    for key in list_keys:
        events = tx_dict.get(key)
        if events is None:
            tx_dict[key] = []
        elif isinstance(events, list):
            cleaned_events = []
            for event in events:
                if not isinstance(event, dict):
                    continue
                for field in hex_int_fields:
                    value = event.get(field)
                    if isinstance(value, str) and value.startswith("0x"):
                        try:
                            event[field] = int(value, 16)
                        except ValueError:
                            pass
                cleaned_events.append(event)
            tx_dict[key] = cleaned_events

    for transfer in tx_dict.get("erc20_transfers", []):
        amount = transfer.get("amount")
        if isinstance(amount, str):
            if amount.startswith("0x"):
                transfer["amount"] = int(amount, 16)
            else:
                transfer["amount"] = int(amount)

        # Preserve original checksum casing

    for transfer in tx_dict.get("internal_transactions", []):
        value = transfer.get("value")
        if value is None:
            continue
        if isinstance(value, str):
            if value.startswith("0x"):
                transfer["value"] = int(value, 16) / 1e18
            else:
                transfer["value"] = float(value)
        else:
            transfer["value"] = float(value)

        # Preserve original checksum casing

    return tx_dict


async def run_builder(args: argparse.Namespace) -> None:
    builder = LiveTokenBuilder()
    processed_provider = builder.processed_tx_provider
    token_provider = builder.token_provider
    start_block, end_block = resolve_block_bounds(processed_provider, args)

    started_at = perf_counter()
    try:
        token = await builder.build_token(
            args.token,
            start_block=start_block,
            end_block=end_block,
            max_blocks=args.max_blocks,
        )
    finally:
        await builder.close()

    metadata = fetch_metadata(args.token)
    apply_metadata(token, metadata)

    replayed_count = 0

    if not token.erc20_transfers:
        if not token.token_data.pool_manager:
            token.token_data._initialize_pool_manager()

        await asyncio.to_thread(
            token_provider.load_blocks_for_token,
            args.token,
            start_block,
            end_block,
        )
        raw_transactions = await asyncio.to_thread(
            token_provider.transactions_for,
            args.token,
        )
        replay_transactions = [
            tx for tx in raw_transactions if start_block <= tx.block_number <= end_block
        ]
        replayed_count = len(replay_transactions)
        for tx in replay_transactions:
            token.update_from_transaction(normalize_transaction(tx.to_dict()))

        # Re-apply metadata after replaying transactions to fill any remaining gaps.
        apply_metadata(token, metadata)

    # Optional verification against metadata total supply
    expected_supply = None
    if metadata and getattr(metadata, "total_supply", None) is not None:
        try:
            expected_supply = int(metadata.total_supply)
        except (TypeError, ValueError):
            expected_supply = None

    actual_supply = token.total_supply or token.token_data.total_supply or 0
    if args.verify_total_supply:
        if expected_supply is None:
            raise AssertionError(
                "Metadata for the token does not expose total_supply; cannot verify."  # noqa: EM101
            )
        if actual_supply == 0:
            raise AssertionError(
                f"Rebuilt token total supply is zero for {args.token}; expected {expected_supply}."
            )
        if actual_supply != expected_supply:
            raise AssertionError(
                f"Total supply mismatch for {args.token}: rebuilt {actual_supply} vs metadata {expected_supply}."
            )

    elapsed = perf_counter() - started_at

    pool_manager = token.pool_manager
    pools = pool_manager.get_all_pools() if pool_manager else []

    print("\nToken snapshot built")
    print("====================")
    print(f"Contract       : {token.contract_address}")
    name = token.name or (getattr(metadata, "name", None) if metadata else None)
    symbol = token.symbol or (getattr(metadata, "symbol", None) if metadata else None)
    decimals_val = token.decimals if token.decimals is not None else (
        getattr(metadata, "decimals", None) if metadata else None
    )
    total_supply_val = actual_supply if actual_supply else expected_supply
    print(f"Name / Symbol  : {name or '-'} / {symbol or '-'}")
    print(f"Decimals       : {decimals_val if decimals_val is not None else '-'}")
    print(f"Latest block   : {token.latest_block_number or '-'}")
    print(f"Creation block : {token.creation_block or '-'}")
    print(f"Total supply   : {fmt_int(total_supply_val)}")
    print(f"Pools tracked  : {len(pools)}")

    if replayed_count:
        print(f"Block range    : [{start_block}, {end_block}]")
        print(f"Processed tx   : {replayed_count}")

    if pools:
        print("\nTop pools (by denomination reserve):")
        sorted_pools = sorted(pools, key=lambda pool: pool.get_denom_reserve(), reverse=True)
        for pool in sorted_pools[:5]:
            address = getattr(pool, "display_address", pool.pool_address)
            denom_symbol = DENOM_ADDRESSES.get(pool.denom_address, "Unknown")
            print(
                f"  {pool.get_protocol():<12} {address[:42]} denom={denom_symbol} "
                f"reserve={pool.get_denom_reserve():.4f} token_reserve={pool.get_token_reserve():.4f}"
            )

    transfer_events = sum(len(events) for events in token.erc20_transfers.values())
    holder_addresses = set()
    for transfers in token.erc20_transfers.values():
        for transfer in transfers:
            holder_addresses.add(transfer["from_address"])
            holder_addresses.add(transfer["to_address"])
    holder_addresses.discard("0x0000000000000000000000000000000000000000")

    print(f"\nERC20 transfers indexed : {fmt_int(transfer_events)}")
    print(f"Unique holder addrs     : {len(holder_addresses)}")
    print(f"Build time              : {elapsed:.2f}s\n")


def main() -> None:
    args = parse_args()
    asyncio.run(run_builder(args))


if __name__ == "__main__":
    main()
