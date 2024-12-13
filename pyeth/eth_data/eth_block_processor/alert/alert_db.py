import lmdb
import msgpack
import os
import asyncio
from typing import List, Dict, Any
from dataclasses import asdict
from general_utils.general_utils.logging.logger import get_logger


logger = get_logger(name="alert_db_logger", log_folder="alert")


class AlertDB:
    """
    AlertDB handles the storage and retrieval of alert data using LMDB.
    It supports asynchronous operations to ensure non-blocking performance.
    """
    def __init__(self, path=None, map_size=1*1024*1024*1024):
        """
        Initializes the AlertDB.

        Args:
            path (str, optional): Path to the LMDB database. Defaults to a path based on environment variable.
            map_size (int, optional): Maximum size of the database. Defaults to 1GB.
        """
        if path is None:
            path = os.path.join(os.environ.get("ETH_ADDRESS_DIR", ""), "alerts", "alert_store.lmdb")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.env = lmdb.open(path, map_size=map_size, max_dbs=0, lock=True, readahead=True, writemap=False)
        self.cache = {}
        self.cache_size = 1000
        self.lock = asyncio.Lock()

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        await self.close()

    async def close(self):
        """
        Flushes the cache and closes the LMDB environment.
        """
        await self.flush_cache()
        self.env.close()

    async def add_alerts(self, alerts: List[Dict[str, Any]]):
        """
        Adds a list of alerts to the cache.

        Args:
            alerts (List[Dict[str, Any]]): List of alert dictionaries to be added.
        """
        async with self.lock:
            for alert in alerts:
                alert_data = asdict(alert)
                alert_type = alert_data.get('alert_type')
                logger.debug(f"Processing alert with alert_type: {alert_type}")

                if not isinstance(alert_type, str):
                    logger.error(f"Invalid alert_type: {alert_type}. Expected a string.")
                    continue  # Skip invalid alert

                if alert_type not in self.cache:
                    self.cache[alert_type] = []
                self.cache[alert_type].append(alert_data)
                logger.debug(f"Added alert to cache under {alert_type}. Cache size: {len(self.cache[alert_type])}")

                if len(self.cache[alert_type]) >= self.cache_size:
                    logger.debug(f"Cache size for {alert_type} reached {self.cache_size}. Flushing cache.")
                    await self.flush_cache(alert_type)

    async def flush_cache(self, alert_type: str = None):
        """
        Flushes the cached alerts to the LMDB database.

        Args:
            alert_type (str, optional): Specific type of alerts to flush. Flushes all if None.
        """
        async with self.lock:
            with self.env.begin(write=True) as txn:
                if alert_type:
                    if not isinstance(alert_type, str):
                        logger.error(f"Invalid alert_type during flush_cache: {alert_type}. Expected a string.")
                        return  # Exit early to prevent errors

                    key = alert_type.encode('utf-8')  # Ensure key is always defined

                    if alert_type in self.cache and self.cache[alert_type]:
                        existing_data = txn.get(key)
                        if existing_data:
                            try:
                                existing_alerts = msgpack.unpackb(existing_data, raw=False)
                                if not isinstance(existing_alerts, list):
                                    logger.error(f"Existing data for {alert_type} is not a list.")
                                    existing_alerts = []
                            except msgpack.exceptions.ExtraData:
                                logger.error(f"ExtraData encountered while unpacking data for {alert_type}.")
                                existing_alerts = []
                            except msgpack.exceptions.FormatError as fe:
                                logger.error(f"Format error unpacking data for {alert_type}: {fe}")
                                existing_alerts = []
                            except Exception as e:
                                logger.error(f"Unexpected error unpacking data for {alert_type}: {e}")
                                existing_alerts = []
                        existing_alerts.extend(self.cache[alert_type])
                    else:
                        existing_alerts = self.cache[alert_type]

                    packed_alerts = msgpack.packb(existing_alerts, use_bin_type=True)
                    txn.put(key, packed_alerts)
                    self.cache[alert_type] = []
                else:
                    # Flush all alert types
                    for alert_type_key, alerts_list in self.cache.items():
                        if not isinstance(alert_type_key, str):
                            logger.error(f"Invalid alert_type_key in cache: {alert_type_key}. Skipping.")
                            continue
                        if alerts_list:
                            key = alert_type_key.encode('utf-8')  # Ensure key is defined
                            existing_data = txn.get(key)
                            try:
                                existing_alerts = msgpack.unpackb(existing_data, raw=False)
                                if not isinstance(existing_alerts, list):
                                    logger.error(f"Existing data for {alert_type_key} is not a list.")
                                    existing_alerts = []
                            except msgpack.exceptions.ExtraData:
                                logger.error(f"ExtraData encountered while unpacking data for {alert_type_key}.")
                                existing_alerts = []
                            except msgpack.exceptions.FormatError as fe:
                                logger.error(f"Format error unpacking data for {alert_type_key}: {fe}")
                                existing_alerts = []
                            except Exception as e:
                                logger.error(f"Unexpected error unpacking data for {alert_type_key}: {e}")
                                existing_alerts = []

                            existing_alerts.extend(alerts_list)
                            packed_alerts = msgpack.packb(existing_alerts, use_bin_type=True)
                            txn.put(key, packed_alerts)
                            self.cache[alert_type_key] = []

    async def get_alerts(self, alert_type: str = None) -> List[Dict[str, Any]]:
        """
        Retrieves alerts from the LMDB database.

        Args:
            alert_type (str, optional): Specific type of alerts to retrieve. Retrieves all if None.

        Returns:
            List[Dict[str, Any]]: List of alert dictionaries.
        """
        async with self.lock:
            with self.env.begin() as txn:
                if alert_type:
                    key = alert_type.encode('utf-8')
                    data = txn.get(key)
                    if data:
                        try:
                            alerts = msgpack.unpackb(data, raw=False)
                            if isinstance(alerts, list):
                                return alerts
                            else:
                                logger.error(f"Unexpected data format for {alert_type}: Expected list, got {type(alerts)}")
                                return []
                        except Exception as e:
                            logger.error(f"Error unpacking data for {alert_type}: {e}")
                            return []
                    else:
                        return []
                else:
                    alerts = []
                    for key, value in txn.cursor():
                        try:
                            unpacked = msgpack.unpackb(value, raw=False)
                            if isinstance(unpacked, list):
                                alerts.extend(unpacked)
                            else:
                                logger.error(f"Unexpected data format for {key.decode('utf-8')}: Expected list, got {type(unpacked)}")
                        except Exception as e:
                            logger.error(f"Error unpacking data for {key.decode('utf-8')}: {e}")
                    return alerts

