#!/usr/bin/env python3
"""
Fetch the next live block published by the token tracker and verify that
PyReth metadata lookups succeed without passing pre-fetched headers.
"""

from __future__ import annotations

import asyncio
import os

import aio_pika
import orjson
from pyreth import chain_query as pyreth_chain_query

RABBITMQ_URL = os.environ.get("RABBITMQ_URL", "amqp://guest:guest@localhost/")
EXCHANGE_NAME = "blocks_exchange"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19d4a2e9eb0cE3606eB48"
MESSAGE_TIMEOUT_SECONDS = float(os.environ.get("BLOCK_MESSAGE_TIMEOUT", "15.0"))


async def fetch_next_published_block() -> int:
    """Listen for the next processed block payload and return its number."""
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

        if block_number is None:
            raise RuntimeError("Published block payload did not include block_number")

        return int(block_number)
    finally:
        await connection.close()


def print_metadata(prefix: str, meta) -> None:
    """Pretty-print a subset of token metadata fields."""
    if meta is None:
        print(f"{prefix}: not an ERC-20 contract")
        return
    print(
        f"{prefix}: name={meta.name} symbol={meta.symbol} decimals={meta.decimals} total_supply={meta.total_supply}"
    )


async def main() -> None:
    print(f"Waiting for next block from '{EXCHANGE_NAME}' on {RABBITMQ_URL} ...")
    block_number = await fetch_next_published_block()
    print(f"Received block {block_number}")

    chain_query = pyreth_chain_query()
    print(f"Querying USDC metadata at block {block_number}...")
    try:
        metadata = chain_query.get_token_metadata(USDC_ADDRESS, block_number)
        print_metadata("Success", metadata)
    except Exception as exc:
        print(f"Metadata lookup failed: {exc}")


if __name__ == "__main__":
    asyncio.run(main())
