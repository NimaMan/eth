import time
import functools
import asyncio
from ethblockprocessor.utils.profiling.performance_logger import PerformanceLogger


def profile_async(category: str):
    def decorator(func):
        @functools.wraps(func)
        async def wrapper(*args, **kwargs):
            start_time = time.perf_counter()  # More precise than time.time()
            result = await func(*args, **kwargs)
            execution_time = time.perf_counter() - start_time
            
            # Asynchronously log without blocking
            asyncio.create_task(
                PerformanceLogger().log_execution_time(
                    category=category,
                    function_name=func.__name__,
                    execution_time=execution_time
                )
            )
            return result
        return wrapper
    return decorator