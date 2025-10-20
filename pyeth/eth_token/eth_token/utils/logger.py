"""
Package-specific logger helpers that delegate to the shared eth_data logger.
"""
from typing import Optional, Any

from eth_data.utils.logger import (
    ETH_LOG_DIR,
    cleanup_empty_logs,
    get_logger as _base_get_logger,
    register_skip_cleanup_folder,
)

# Preserve block_processor logs used by long-running services.
register_skip_cleanup_folder("block_processor")


def get_logger(
    name: str = "baygus",
    log_folder: Optional[str] = "baygus",
    base_log_dir: Optional[str] = None,
    console_output: bool = False,
    **kwargs: Any,
):
    """
    Return a logger configured by the central eth_data implementation.
    """
    return _base_get_logger(
        name=name,
        log_folder=log_folder,
        base_log_dir=base_log_dir,
        console_output=console_output,
        **kwargs,
    )


__all__ = [
    "ETH_LOG_DIR",
    "get_logger",
    "cleanup_empty_logs",
    "register_skip_cleanup_folder",
]
