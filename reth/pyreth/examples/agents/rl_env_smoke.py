#!/usr/bin/env python3
"""
Compact smoke test for the Rust RL env via pyreth.
- Uses a default local Reth datadir (override with env RETH_DATADIR)
- Runs one Buy then one Sell and prints state + prices
"""
import os
import pyreth as pr

DEFAULT_RETH_DATA_DIR = os.environ.get("RETH_DATADIR", "/home/nima/.local/share/reth/mainnet")
AGENT = os.environ.get("BAYGUS_TEST_EOA", "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")
START_BLOCK = int(os.environ.get("START_BLOCK", "23311982"))
TIP_GWEI = int(os.environ.get("TIP_GWEI", "1"))
SLIPPAGE_BPS = int(os.environ.get("SLIPPAGE_BPS", "50"))

UNIV3_USDC_500 = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"

def main():
    # Note: PyStablecoinEnv signature is (agent, start_block, reth_datadir=None, tip_gwei=1, slippage_bps=50, block_step=1)
    env = pr.PyStablecoinEnv(
        AGENT,
        START_BLOCK,
        reth_datadir=DEFAULT_RETH_DATA_DIR,
        tip_gwei=TIP_GWEI,
        slippage_bps=SLIPPAGE_BPS,
        block_step=1,
    )
    chain, pf = env.state()
    print(f"S @ block={chain.block} base_fee_wei={chain.base_fee_wei} portfolio: ETH={pf.eth_wei} USDC={pf.usdc_raw} USDT={pf.usdt_raw} DAI={pf.dai_raw}")

    # BUY 0.05 ETH -> USDC on UniV3 500
    buy = pr.PyStablecoinAction("univ3", UNIV3_USDC_500, "Buy", "USDC", 50_000_000_000_000_000, fee=500)
    out1 = env.step(buy)
    print(f"Buy reward={out1.reward:.6f} info={out1.info}")
    print(f"S1 @ block={out1.chain.block} portfolio: ETH={out1.portfolio.eth_wei} USDC={out1.portfolio.usdc_raw}")

    # SELL half of USDC (min 10 USDC)
    usdc_bal = int(out1.portfolio.usdc_raw)
    sell_amount = max(usdc_bal // 2, 10 * 10**6)
    if sell_amount > 0:
        sell = pr.PyStablecoinAction("univ3", UNIV3_USDC_500, "Sell", "USDC", sell_amount, fee=500)
        out2 = env.step(sell)
        print(f"Sell reward={out2.reward:.6f} info={out2.info}")
        print(f"S2 @ block={out2.chain.block} portfolio: ETH={out2.portfolio.eth_wei} USDC={out2.portfolio.usdc_raw}")
    else:
        print("No USDC to sell; skipping sell step")

if __name__ == "__main__":
    main()
