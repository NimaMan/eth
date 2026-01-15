"""Shared function and event signatures sourced from PyReth."""
import pyreth

FUNCTION_SIGNATURES = dict(pyreth.function_signatures())
EVENT_TOPICS = dict(pyreth.event_topics())
EVENT_TOPICS_REVERSE = dict(pyreth.event_topics_reverse())
CRITICAL_SCAM_FUNCTIONS = set(pyreth.critical_scam_function_selectors())
UNISWAP_CONTRACTS = dict(pyreth.uniswap_contracts())

__all__ = [
    "FUNCTION_SIGNATURES",
    "EVENT_TOPICS",
    "EVENT_TOPICS_REVERSE",
    "CRITICAL_SCAM_FUNCTIONS",
    "UNISWAP_CONTRACTS",
]
