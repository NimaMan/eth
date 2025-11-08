"""
Quick utility for validating how fast we observe new Ethereum blocks.

The script:
1. Subscribes to `newHeads` over the configured WebSocket endpoint.
2. Logs each head arrival with both the chain timestamp and the local receive time.
3. Sleeps for a second, then tries to fetch the full block over HTTP to see if it is
   already available, logging any lag.

Run it for ~60 seconds (default) to compare the cadence you receive vs. the expected
12s target and to capture how quickly your node serves up full block data.
"""

from __future__ import annotations

import argparse
import asyncio
import datetime as dt
import statistics
from collections import defaultdict
from typing import Dict, List, Optional

import aio_pika
from aio_pika.exceptions import QueueEmpty

from web3 import AsyncWeb3
from web3.exceptions import BlockNotFound
from web3.providers import AsyncHTTPProvider, WebSocketProvider
from eth_data.utils.logger import get_logger


logger = get_logger(name="block_arrival_monitor", console_output=True)


def _decode_int(value) -> int:
    """Convert either hex-string or int to int."""
    if isinstance(value, int):
        return value
    return int(value, 16)


async def monitor_block_arrivals(
    websocket_url: str,
    http_url: str,
    duration_seconds: int = 60,
    rabbitmq_url: str = "amqp://guest:guest@localhost/",
) -> None:
    """Subscribe to newHeads and log arrival + fetch latencies for roughly duration_seconds."""
    ws_w3 = AsyncWeb3(WebSocketProvider(websocket_url))
    http_w3 = AsyncWeb3(AsyncHTTPProvider(http_url))
    start = asyncio.get_running_loop().time()
    last_arrival: Optional[dt.datetime] = None
    subscription_id: Optional[str] = None
    arrival_lags: List[float] = []
    intervals: List[float] = []
    fetch_latencies: List[float] = []
    publish_latencies: List[float] = []
    arrival_times: Dict[int, dt.datetime] = {}
    pending_publish: Dict[int, List[dt.datetime]] = defaultdict(list)

    stop_event = asyncio.Event()

    async def head_monitor() -> None:
        nonlocal last_arrival, subscription_id
        async with ws_w3:
            if not await ws_w3.is_connected():
                raise ConnectionError(f"Failed to connect to WebSocket {websocket_url}")
            subscription_id = await ws_w3.eth.subscribe("newHeads")
            logger.info("Subscribed to newHeads with ID %s", subscription_id)
            stream = ws_w3.socket.process_subscriptions()
            while not stop_event.is_set():
                elapsed = asyncio.get_running_loop().time() - start
                remaining = duration_seconds - elapsed
                if remaining <= 0:
                    logger.info(
                        "Duration %.1fs elapsed without additional heads, exiting",
                        duration_seconds,
                    )
                    break
                try:
                    message = await asyncio.wait_for(stream.__anext__(), timeout=remaining)
                except asyncio.TimeoutError:
                    logger.warning(
                        "No new heads received for %.1fs, exiting monitor",
                        duration_seconds,
                    )
                    break

                block_data = message.get("result")
                if not block_data or "number" not in block_data or "timestamp" not in block_data:
                    continue

                arrival_time = dt.datetime.now(dt.timezone.utc)
                block_number = _decode_int(block_data["number"])
                chain_timestamp = _decode_int(block_data["timestamp"])
                chain_time = dt.datetime.fromtimestamp(chain_timestamp, dt.timezone.utc)
                since_last = (
                    (arrival_time - last_arrival).total_seconds() if last_arrival else None
                )
                lag_seconds = (arrival_time - chain_time).total_seconds()
                arrival_lags.append(lag_seconds)
                arrival_times[block_number] = arrival_time

                pending_times = pending_publish.pop(block_number, [])
                for pub_time in pending_times:
                    publish_latencies.append((pub_time - arrival_time).total_seconds())

                if since_last is None:
                    logger.info(
                        "Head %s arrived: chain=%s local=%s lag=%.2fs",
                        block_number,
                        chain_time.isoformat(),
                        arrival_time.isoformat(),
                        lag_seconds,
                    )
                else:
                    logger.info(
                        "Head %s arrived: chain=%s local=%s lag=%.2fs interval=%.2fs",
                        block_number,
                        chain_time.isoformat(),
                        arrival_time.isoformat(),
                        lag_seconds,
                        since_last,
                    )
                    intervals.append(since_last)
                last_arrival = arrival_time

                await asyncio.sleep(1)

                try:
                    fetch_start = dt.datetime.now(dt.timezone.utc)
                    block = await http_w3.eth.get_block(block_number, full_transactions=False)
                    fetch_end = dt.datetime.now(dt.timezone.utc)
                    since_arrival = (fetch_end - arrival_time).total_seconds()
                    fetch_duration = (fetch_end - fetch_start).total_seconds()
                    fetch_latencies.append(since_arrival)
                    tx_count = len(block.get("transactions", []))
                    logger.info(
                        "Fetched block %s (%d txs) after %.2fs (HTTP %.2fs)",
                        block_number,
                        tx_count,
                        since_arrival,
                        fetch_duration,
                    )
                except BlockNotFound:
                    logger.warning(
                        "Block %s not yet available via HTTP after 1s wait", block_number
                    )
                except Exception as exc:  # noqa: BLE001
                    logger.error("Error fetching block %s: %s", block_number, exc)

    async def publish_monitor() -> None:
        try:
            connection = await aio_pika.connect_robust(rabbitmq_url)
            channel = await connection.channel()
            exchange = await channel.declare_exchange(
                "blocks_exchange", aio_pika.ExchangeType.FANOUT, durable=True
            )
            queue = await channel.declare_queue(exclusive=True, auto_delete=True)
            await queue.bind(exchange, routing_key="processed_blocks")
        except Exception as exc:
            logger.error("Failed to connect to RabbitMQ: %s", exc)
            return

        try:
            while not stop_event.is_set():
                elapsed = asyncio.get_running_loop().time() - start
                remaining = duration_seconds - elapsed
                if remaining <= 0:
                    break
                try:
                    message = await asyncio.wait_for(queue.get(), timeout=remaining)
                except asyncio.TimeoutError:
                    break
                except QueueEmpty:
                    continue
                async with message.process(ignore_processed=True):
                    publish_time = dt.datetime.now(dt.timezone.utc)
                    headers = message.headers or {}
                    block_number = headers.get("block_number")
                    if block_number is None:
                        continue
                    block_number = int(block_number)
                    arrival_time = arrival_times.get(block_number)
                    if arrival_time:
                        publish_latencies.append((publish_time - arrival_time).total_seconds())
                    else:
                        pending_publish[block_number].append(publish_time)
        finally:
            try:
                await channel.close()
            except Exception:
                pass
            await connection.close()

    try:
        head_task = asyncio.create_task(head_monitor())
        publish_task = asyncio.create_task(publish_monitor())
        await asyncio.gather(head_task, publish_task)
    finally:
        stop_event.set()
        if subscription_id:
            try:
                await ws_w3.eth.unsubscribe(subscription_id)
            except Exception:  # noqa: BLE001
                pass
        if hasattr(ws_w3.provider, "disconnect"):
            await ws_w3.provider.disconnect()
        if hasattr(http_w3.provider, "close"):
            await http_w3.provider.close()

        if pending_publish:
            logger.warning(
                "Pending RabbitMQ messages without arrival timestamps: %s", list(pending_publish)[:5]
            )

        def summarize(label: str, samples: List[float]) -> None:
            if not samples:
                logger.warning("No samples collected for %s", label.lower())
                return
            logger.info(
                "%s stats samples=%d mean=%.2fs median=%.2fs min=%.2fs max=%.2fs stdev=%.2fs",
                label,
                len(samples),
                statistics.fmean(samples),
                statistics.median(samples),
                min(samples),
                max(samples),
                statistics.pstdev(samples),
            )

        summarize("Arrival lag", arrival_lags)
        summarize("Interval", intervals)
        summarize("Fetch wait", fetch_latencies)
        summarize("Publish delay", publish_latencies)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Monitor live block arrival timings.")
    parser.add_argument(
        "--websocket-url",
        default="ws://127.0.0.1:8546",
        help="WebSocket endpoint for newHeads subscription",
    )
    parser.add_argument(
        "--http-url",
        default="http://127.0.0.1:8545",
        help="HTTP endpoint for fetching full blocks",
    )
    parser.add_argument(
        "--duration",
        type=int,
        default=60,
        help="How many seconds to run before exiting",
    )
    parser.add_argument(
        "--rabbitmq-url",
        default="amqp://guest:guest@localhost/",
        help="RabbitMQ connection string for processed block stream",
    )
    return parser.parse_args()


async def _async_main() -> None:
    args = parse_args()
    await monitor_block_arrivals(
        websocket_url=args.websocket_url,
        http_url=args.http_url,
        duration_seconds=args.duration,
        rabbitmq_url=args.rabbitmq_url,
    )


def main() -> None:
    asyncio.run(_async_main())


if __name__ == "__main__":
    main()
