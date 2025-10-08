"""
Configuration module for ERC20 token analysis.
"""

from .scam_thresholds import (
    THRESHOLDS,
    TOKEN_TO_CATEGORY,
    get_threshold_for_token,
)

__all__ = [
    'THRESHOLDS',
    'TOKEN_TO_CATEGORY',
    'get_threshold_for_token',
]