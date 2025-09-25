#!/usr/bin/env python3
"""
Prior Tx → Buy/Approve/Sell Example

Demonstrates running a prior transaction before the pool buy/approve/sell
viability sequence. The prior tx can be provided as:

1) A real processed transaction (by hash), or
2) A minimal unsigned transaction (from/to/value/data/nonce)

Environment variables (optional):
  TOKEN_ADDRESS, POOL_ADDRESS         # defaults to USDC/WETH V2
  PRIOR_TX_HASH                       # if set, uses processed prior tx
  PRIOR_UNSIGNED_FROM, PRIOR_UNSIGNED_TO, PRIOR_UNSIGNED_VALUE_WEI,
  PRIOR_UNSIGNED_DATA_HEX, PRIOR_UNSIGNED_NONCE

Usage:
  python prior_tx_then_trade.py
  PRIOR_TX_HASH=0x... python prior_tx_then_trade.py
  PRIOR_UNSIGNED_FROM=0x... PRIOR_UNSIGNED_TO=0x... PRIOR_UNSIGNED_VALUE_WEI=0 \
    PRIOR_UNSIGNED_DATA_HEX=0x... python prior_tx_then_trade.py
"""

import os
import pyreth


def main():
    # Defaults: USDC/WETH Uniswap V2
    token = os.environ.get("TOKEN_ADDRESS", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
    pool = os.environ.get("POOL_ADDRESS",  "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")

    print("=" * 80)
    print("Prior Tx → Buy/Approve/Sell Viability")
    print("=" * 80)
    print(f"Token: {token}")
    print(f"Pool:  {pool} (Uniswap V2)")

    # Init components
    reth = pyreth.PyReth()
    sim = reth.pool_buy_sell_simulator()
    txp = reth.tx_processor()

    # Base config
    cfg = pyreth.PoolViabilityConfig()
    cfg.test_amount_eth = 0.01
    # USDC has 6 decimals
    if token.lower() == "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48":
        cfg.token_decimals = 6

    # Option A: real prior tx (processed)
    prior_hash = os.environ.get("PRIOR_TX_HASH")
    if prior_hash:
        try:
            print(f"Using prior processed tx: {prior_hash}")
            prior = txp.process_transaction_from_hash_with_simulation(prior_hash)
            cfg.set_prior_tx_from_processed(prior)
        except Exception as e:
            print(f"Warning: failed to load prior tx {prior_hash}: {e}")

    # Option B: unsigned prior tx
    prior_from = os.environ.get("PRIOR_UNSIGNED_FROM")
    if prior_from and not prior_hash:
        prior_to = os.environ.get("PRIOR_UNSIGNED_TO")
        val_wei = os.environ.get("PRIOR_UNSIGNED_VALUE_WEI")
        data_hex = os.environ.get("PRIOR_UNSIGNED_DATA_HEX")
        nonce = os.environ.get("PRIOR_UNSIGNED_NONCE")
        try:
            cfg.set_prior_tx_from_unsigned(
                prior_from,
                prior_to,
                None if val_wei is None else hex(int(val_wei)),
                data_hex,
                None if nonce is None else int(nonce),
            )
            print("Using unsigned prior tx (from env params)")
        except Exception as e:
            print(f"Warning: failed to set unsigned prior tx: {e}")

    print("\nRunning viability with optional prior tx...")
    res = sim.check_uniswap_v2_pool(token_address=token, pool_address=pool, config=cfg)

    print("\nResults:")
    print(f"  Can Buy:     {'✅' if res.can_buy else '❌'}")
    print(f"  Can Approve: {'✅' if res.can_approve else '❌'}")
    print(f"  Can Sell:    {'✅' if res.can_sell else '❌'}")
    print(f"  Buy Tax:     {res.buy_tax_percentage:.4f}%")
    print(f"  Sell Tax:    {res.sell_tax_percentage:.4f}%")
    if res.error_message:
        print(f"  Error:       {res.error_message}")

    tradeable = res.can_buy and res.can_approve and res.can_sell
    print(f"\nFinal: {'✅ TRADEABLE' if tradeable else '❌ NOT TRADEABLE'}")


if __name__ == "__main__":
    main()

