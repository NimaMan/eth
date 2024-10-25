import time
import functools
from ethblockprocessor.utils.logger import get_logger


logger = get_logger("profiler_blcok_processor")


def profile(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        start_time = time.time()
        result = func(*args, **kwargs)
        end_time = time.time()
        execution_time = end_time - start_time
        logger.info(f"{execution_time:.4f} --->F '{func.__name__}' ")
        return result
    return wrapper

