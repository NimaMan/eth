from eth_block_processor.utils.logger import get_logger
import asyncio
import json
from typing import Dict, Any, List
import aiofiles
from collections import defaultdict
import time
import os

class PerformanceLogger:
    _instance = None
    _lock = asyncio.Lock()
    BATCH_SIZE = 100
    FLUSH_INTERVAL = 5  # seconds

    def __init__(self):
        if not hasattr(self, 'initialized'):
            self.logger = get_logger("profiler")
            self.metrics_buffer = defaultdict(list)
            self.last_flush = time.perf_counter()
            self.initialized = True
            self._ensure_log_directory()
            asyncio.create_task(self._periodic_flush())

    def _ensure_log_directory(self):
        os.makedirs('logs', exist_ok=True)

    async def log_execution_time(self, category: str, function_name: str, 
                               execution_time: float, extra_data: Dict[str, Any] = None):
        metric = {
        'function': function_name,
        'execution_time': round(execution_time, 4),
        **(extra_data or {})
        }
        
        async with self._lock:
            self.metrics_buffer[category].append(metric)
            
            if len(self.metrics_buffer[category]) >= self.BATCH_SIZE:
                await self._flush_category(category)

    async def _periodic_flush(self):
        while True:
            try:
                await asyncio.sleep(self.FLUSH_INTERVAL)
                await self._flush_all()
            except Exception as e:
                self.logger.error(f"Error in periodic flush: {e}")

    async def _flush_all(self):
        async with self._lock:
            for category in list(self.metrics_buffer.keys()):
                await self._flush_category(category)

    async def _flush_category(self, category: str):
        try:
            metrics = self.metrics_buffer[category]
            if not metrics:
                return
                
            async with aiofiles.open(f'logs/profiler_{category}.jsonl', 'a') as f:
                for metric in metrics:
                    await f.write(json.dumps(metric) + '\n')  # Add newline after each entry
            
            self.metrics_buffer[category] = []
        except Exception as e:
            self.logger.error(f"Error flushing category {category}: {e}")