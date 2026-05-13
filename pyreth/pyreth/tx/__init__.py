"""Transaction analysis tools.

Usage:
    from pyreth.tx import analyze_transaction
    result = analyze_transaction("0x...")
"""
from __future__ import annotations

from typing import Any

from pyreth import tx_processor as _tx_processor


def analyze_transaction(tx_hash: str) -> dict[str, Any]:
    """Analyze a transaction and return a structured summary."""
    processor = _tx_processor()

    try:
        processed = processor.process_transaction_from_hash_with_simulation(tx_hash)
    except Exception as e:
        return {"tx_hash": tx_hash, "error": f"Failed to process transaction: {e}"}

    actions = list(processed.actions) if processed.actions else []
    token_transfers = []

    if hasattr(processed, "token_transfers") and processed.token_transfers:
        for t in processed.token_transfers:
            token_transfers.append({
                "token": getattr(t, "token_address", None) or getattr(t, "token", None),
                "from": getattr(t, "from_address", None) or getattr(t, "sender", None),
                "to": getattr(t, "to_address", None) or getattr(t, "recipient", None),
                "amount": getattr(t, "amount", None),
            })

    return {
        "tx_hash": tx_hash,
        "status": processed.status,
        "block": processed.block_number,
        "from": processed.from_address,
        "to": processed.to_address,
        "value_eth": str(processed.value) if hasattr(processed, "value") else None,
        "gas_used": getattr(processed, "gas_used", None),
        "actions": actions,
        "token_transfers": token_transfers,
        "summary": _summarize(processed, actions),
    }


def get_processed_transaction(tx_hash: str) -> dict[str, Any]:
    """Get raw processed transaction data."""
    processor = _tx_processor()
    try:
        processed = processor.process_transaction_from_hash_with_simulation(tx_hash)
    except Exception as e:
        return {"tx_hash": tx_hash, "error": f"Failed to process transaction: {e}"}

    return {
        "tx_hash": tx_hash,
        "status": processed.status,
        "block_number": processed.block_number,
        "from_address": processed.from_address,
        "to_address": processed.to_address,
        "gas_used": getattr(processed, "gas_used", None),
        "gas_price": getattr(processed, "gas_price", None),
        "actions": list(processed.actions) if processed.actions else [],
        "value": str(processed.value) if hasattr(processed, "value") else None,
    }


def _summarize(processed, actions: list[str]) -> str:
    parts = []
    if not actions:
        return "Unknown transaction type"

    lowered = [a.lower() for a in actions]
    if "swap" in lowered:
        parts.append("Token swap")
    if "transfer" in lowered:
        parts.append("Token transfer")
    if "approve" in lowered:
        parts.append("Token approval")
    if "mint" in lowered:
        parts.append("Liquidity provision")
    if "burn" in lowered:
        parts.append("Liquidity removal")

    return "; ".join(parts) if parts else f"Contract interaction: {', '.join(actions)}"
