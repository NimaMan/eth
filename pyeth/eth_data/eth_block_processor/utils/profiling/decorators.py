import time
import functools
import asyncio
from eth_block_processor.utils.profiling.performance_logger import PerformanceLogger
from eth_block_processor.utils.logger import get_logger


logger = get_logger(name="block_processor", log_folder="eth_block_processor")


from functools import wraps
import cProfile
import pstats
import io
from time import time

def detailed_profiler(func):
    """
    Detailed profiling decorator that provides function-level timing statistics
    with proper cleanup
    """
    @wraps(func)
    async def wrapper(*args, **kwargs):
        pr = cProfile.Profile()
        try:
            pr.enable()
            start_time = time()
            result = await func(*args, **kwargs)
            end_time = time()
            
        finally:
            try:
                pr.disable()
                # Get detailed stats
                s = io.StringIO()
                ps = pstats.Stats(pr, stream=s).sort_stats('cumulative')
                ps.print_stats(10)  # Print top 10 time-consuming functions

                logger.info(f"\nDetailed profiling for {func.__name__}:")
                logger.info(f"Total time: {end_time - start_time:.3f}s")
                logger.info(f"Function breakdown:\n{s.getvalue()}")
                
            except Exception as e:
                logger.error(f"Error in profiler cleanup: {e}")
                
        return result
    return wrapper


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