from collections import defaultdict
import time
import random
import asyncio
from typing import Dict, Any

class MetricsCollector:
    def __init__(self, sampling_rate: float = 0.1):
        self.sampling_rate = sampling_rate
        self.metrics = defaultdict(list)
        self._lock = asyncio.Lock()
    
    async def should_sample(self) -> bool:
        return random.random() < self.sampling_rate
    
    async def record_metric(self, category: str, name: str, value: float):
        if not await self.should_sample():
            return
            
        async with self._lock:
            self.metrics[category].append({
                'timestamp': time.perf_counter(),
                'name': name,
                'value': value
            })
    
    def get_stats(self, category: str) -> Dict[str, float]:
        if not self.metrics[category]:
            return {}
            
        values = [m['value'] for m in self.metrics[category]]
        return {
            'avg': sum(values) / len(values),
            'min': min(values),
            'max': max(values),
            'count': len(values),
            'p95': sorted(values)[int(len(values) * 0.95)]
        }