#!/usr/bin/env python3
"""
Compare address balance changes between:
  - Python eth_data (ProcessedTxProvider.state_changes)
  - PyReth tx_processor (address_balance_changes)

Usage:
  python python_vs_pyreth_address_balance_changes.py <TX_HASH> [--rpc http://127.0.0.1:8545]

Prints only the differences (ETH and token deltas) per address.
Treats WETH as ETH in both.
"""

import os
import sys
import argparse
from decimal import Decimal, getcontext
from typing import Dict, Any


getcontext().prec = 50


def d(x) -> Decimal:
    try:
        return Decimal(str(x))
    except Exception:
        return Decimal(0)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("tx_hash")
    parser.add_argument("--rpc", default=os.environ.get("ETH_RPC", "http://127.0.0.1:8545"))
    args = parser.parse_args()

    # Make local packages importable when running from repo root
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../../"))
    sys.path.extend([os.path.join(repo_root, "py"), os.path.join(repo_root, "py/eth_data")])

    from web3 import Web3
    from eth_data.tx_provider.processed_tx_providor import ProcessedTxProvider
    try:
        from eth_data.chain_utils.common_addresses import ERC20_TOKEN_DECIMALS
    except Exception:
        ERC20_TOKEN_DECIMALS = {"ETH": 18, "WETH": 18}

    # Python eth_data
    w3 = Web3(Web3.HTTPProvider(args.rpc))
    py_provider = ProcessedTxProvider(w3=w3, calculate_state_changes=True)
    py_tx = py_provider.get_processed_tx(args.tx_hash)
    py_changes: Dict[str, Any] = getattr(py_tx, "state_changes", {}) or {}

    # Normalize: currencies (ETH already in ETH units), tokens raw
    py_norm: Dict[str, Dict[str, Any]] = {}
    for addr, ch in py_changes.items():
        cur = ch.get("currency_net") or {}
        tok = ch.get("token_net") or {}
        # Treat WETH as ETH
        eth_val = d(cur.get("ETH", 0)) + d(cur.get("WETH", 0))
        py_norm[addr] = {
            "eth": eth_val,
            "tokens": {t: d(v) for t, v in tok.items()},
        }

    # PyReth
    import pyreth

    reth = pyreth.PyReth()
    proc = reth.tx_processor()
    pr_tx = proc.process_transaction_from_hash_with_simulation(args.tx_hash)

    pr_norm: Dict[str, Dict[str, Any]] = {}
    for addr, ch in getattr(pr_tx, "address_balance_changes", {}).items():
        cur = getattr(ch, "currency_net", {}) or {}
        tok = getattr(ch, "token_net", {}) or {}
        # Convert currencies to display units using decimals map
        eth = Decimal(0)
        for sym, raw in cur.items():
            sym_key = "ETH" if sym == "WETH" else sym
            decs = ERC20_TOKEN_DECIMALS.get(sym_key, 0)
            val = d(raw) / (Decimal(10) ** decs) if decs else d(raw)
            if sym_key == "ETH":
                eth += val
        pr_norm[addr] = {
            "eth": eth,
            "tokens": {t: d(v) for t, v in tok.items()},
        }

    # Compare and print only differences
    all_addrs = set(py_norm.keys()) | set(pr_norm.keys())
    diffs_found = False
    for addr in sorted(all_addrs):
        p = py_norm.get(addr, {"eth": Decimal(0), "tokens": {}})
        r = pr_norm.get(addr, {"eth": Decimal(0), "tokens": {}})
        eth_diff = p["eth"] - r["eth"]
        tok_keys = set(p["tokens"].keys()) | set(r["tokens"].keys())
        tok_diffs = []
        for t in sorted(tok_keys):
            pv, rv = p["tokens"].get(t, Decimal(0)), r["tokens"].get(t, Decimal(0))
            if pv != rv:
                tok_diffs.append((t, pv, rv, pv - rv))
        if eth_diff != 0 or tok_diffs:
            diffs_found = True
            print(f"\nAddress {addr}:")
            if eth_diff != 0:
                print(f"  ETH: python={p['eth']:+.18f} | pyreth={r['eth']:+.18f} | diff={eth_diff:+.18f}")
            for t, pv, rv, dv in tok_diffs:
                print(f"  Token {t}: python={pv} | pyreth={rv} | diff={dv}")

    if not diffs_found:
        print("No differences.")


if __name__ == "__main__":
    main()
