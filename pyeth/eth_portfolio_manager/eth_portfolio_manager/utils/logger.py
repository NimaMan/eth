"""
Utility wrappers for portfolio-manager logging that rely on the shared eth_data logger.
"""
import logging
from typing import Optional, Any

from eth_data.utils.logger import (
    ETH_LOG_DIR,
    cleanup_empty_logs,
    get_logger as _base_get_logger,
    register_skip_cleanup_folder,
)

# Retain long-running service logs that live under this package.
register_skip_cleanup_folder("portfolio_manager")

DEFAULT_TIMESTAMP = "%Y%m%d_%H%M"
MONITOR_TIMESTAMP = "%Y%m%d_%H"
DEFAULT_FORMAT = "%(asctime)s - %(levelname)s - %(message)s"


def get_logger(
    name: str = "portfolio_manager",
    log_folder: Optional[str] = "portfolio_manager",
    base_log_dir: Optional[str] = None,
    console_output: bool = False,
    *,
    timestamp_format: str = DEFAULT_TIMESTAMP,
    **kwargs: Any,
) -> logging.Logger:
    """
    Return a logger using the central eth_data configuration while keeping legacy defaults.
    """
    return _base_get_logger(
        name=name,
        log_folder=log_folder,
        base_log_dir=base_log_dir,
        console_output=console_output,
        timestamp_format=timestamp_format,
        **kwargs,
    )


def get_monitoring_logger(
    name: str = "portfolio_monitor",
    log_folder: Optional[str] = "portfolio_monitor",
    base_log_dir: Optional[str] = None,
    console_output: bool = False,
    **kwargs: Any,
) -> logging.Logger:
    """
    Configure a monitoring logger with console output and hourly log file naming.
    """
    logger = _base_get_logger(
        name=name,
        log_folder=log_folder,
        base_log_dir=base_log_dir,
        console_output=False,
        timestamp_format=MONITOR_TIMESTAMP,
        **kwargs,
    )

    _ensure_console_handler(logger, console_output)
    return logger


def setup_flask_logger(
    app,
    name: str = "portfolio_monitor",
    log_folder: Optional[str] = "portfolio_monitor",
    base_log_dir: Optional[str] = None,
    console_output: bool = False,
    **kwargs: Any,
) -> logging.Logger:
    """
    Configure the Flask app logger using the shared implementation.
    """
    logger = _base_get_logger(
        name=name or app.logger.name,
        log_folder=log_folder,
        base_log_dir=base_log_dir,
        console_output=console_output,
        timestamp_format=MONITOR_TIMESTAMP,
        **kwargs,
    )
    return logger


def _ensure_console_handler(logger: logging.Logger, force_console: bool) -> None:
    """
    Attach a console handler if one is not already present or if explicit output requested.
    """
    has_console = any(isinstance(handler, logging.StreamHandler) for handler in logger.handlers)
    if force_console or not has_console:
        formatter = logging.Formatter(DEFAULT_FORMAT)
        console_handler = logging.StreamHandler()
        console_handler.setFormatter(formatter)
        logger.addHandler(console_handler)


__all__ = [
    "ETH_LOG_DIR",
    "cleanup_empty_logs",
    "get_logger",
    "get_monitoring_logger",
    "register_skip_cleanup_folder",
    "setup_flask_logger",
]
