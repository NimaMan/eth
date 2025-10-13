#!/usr/bin/env python3
"""
Fetch the next live block published by the token tracker and verify that
PyReth metadata lookups succeed when the provided sealed header is threaded
through.
"""

from __future__ import annotations

import asyncio
import os
from typing import Optional, Tuple

import aio_pika
import orjson
import pyreth

RABBITMQ_URL = os.environ.get("RABBITMQ_URL", "amqp://guest:guest@localhost/")
EXCHANGE_NAME = "blocks_exchange"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19d4a2e9eb0cE3606eB48"
MESSAGE_TIMEOUT_SECONDS = float(os.environ.get("BLOCK_MESSAGE_TIMEOUT", "15.0"))


async def fetch_next_published_block() -> Tuple[int, Optional[str]]:
    """Listen for the next processed block payload and return its number/header JSON."""
    connection = await aio_pika.connect_robust(RABBITMQ_URL)
    try:
        channel = await connection.channel()
        await channel.set_qos(prefetch_count=1)

        exchange = await channel.declare_exchange(
            EXCHANGE_NAME,
            aio_pika.ExchangeType.FANOUT,
            durable=True,
        )
        queue = await channel.declare_queue(
            exclusive=True,
            auto_delete=True,
            arguments={"x-max-length": 1, "x-overflow": "drop-head"},
        )
        await queue.bind(exchange, routing_key="")

        queue_iter = queue.iterator()
        async with queue_iter:
            try:
                message = await asyncio.wait_for(
                    queue_iter.__anext__(), timeout=MESSAGE_TIMEOUT_SECONDS
                )
            except asyncio.TimeoutError as exc:
                raise RuntimeError(
                    f"No block published within {MESSAGE_TIMEOUT_SECONDS} seconds"
                ) from exc

            async with message.process():
                payload = orjson.loads(message.body)
                block_number = payload.get("block_number")
                header_json = payload.get("block_header")

        if block_number is None:
            raise RuntimeError("Published block payload did not include block_number")

        return int(block_number), header_json
    finally:
        await connection.close()


def print_metadata(prefix: str, meta) -> None:
    """Pretty-print a subset of token metadata fields."""
    print(
        f"{prefix}: name={meta.name} symbol={meta.symbol} decimals={meta.decimals} total_supply={meta.total_supply}"
    )


async def main() -> None:
    print(f"Waiting for next block from '{EXCHANGE_NAME}' on {RABBITMQ_URL} ...")
    block_number, header_json = await fetch_next_published_block()
    has_header = header_json is not None
    print(f"Received block {block_number} (header included: {has_header})")

    chain_query = pyreth.PyReth().chain_query()
    print("Trying with sealed header JSON from live publisher...")
    meta_with_header = chain_query.get_token_metadata(
        USDC_ADDRESS, block_number, header_json
    )
    if meta_with_header is not None:
        print_metadata("Success with header", meta_with_header)

    print(f"Querying USDC metadata at block {block_number} without header...")
    try:
        meta_without_header = chain_query.get_token_metadata(USDC_ADDRESS, block_number)
        print_metadata("Success without header", meta_without_header)
    except Exception as exc:
        print(f"Call without header failed: {exc}")

    if not has_header:
        print("No header JSON in published payload; skipping retry with header.")
        return


if __name__ == "__main__":
    asyncio.run(main())
