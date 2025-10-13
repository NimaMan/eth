#!/usr/bin/env python3
"""
USDC 1 ETH Buy/Approve/Sell Simulation

This example performs a complete trading simulation with 1 ETH:
1. Buy USDC with 1 ETH
2. Approve router to spend USDC
3. Sell USDC back to ETH

Shows detailed transaction flow and calculates net costs.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import pyreth


def format_usdc_amount(amount: int) -> str:
    """Format USDC amount (6 decimals) to readable string"""
    if amount == 0:
        return "0 USDC"
    # USDC has 6 decimals
    usdc_value = amount / 1_000_000
    return f"{usdc_value:,.2f} USDC"


def format_eth_amount(amount: int) -> str:
    """Format ETH amount (18 decimals) to readable string"""
    if amount == 0:
        return "0 ETH"
    # ETH has 18 decimals
    eth_value = amount / 1e18
    return f"{eth_value:.6f} ETH"


def main():
    print("=" * 80)
    print("USDC Buy/Approve/Sell Simulation with 1 ETH")
    print("=" * 80)
    print()
    
    # USDC and pool configuration
    USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    USDC_WETH_V2_POOL = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    TEST_AMOUNT_ETH = 1.0  # 1 ETH
    
    print("Configuration:")
    print(f"  Token:       USDC ({USDC_ADDRESS})")
    print(f"  Pool:        Uniswap V2 USDC/WETH")
    print(f"  Pool Addr:   {USDC_WETH_V2_POOL}")
    print(f"  Test Amount: {TEST_AMOUNT_ETH} ETH")
    print(f"  Decimals:    USDC=6, WETH=18")
    print()
    
    try:
        # Initialize PyReth and simulator
        reth = pyreth.PyReth()
        simulator = reth.pool_buy_sell_simulator()
        print("✅ Pool Buy Sell Simulator initialized")
        
        # Create custom config for 1 ETH
        config = pyreth.PoolBuySellParameters()
        config.test_amount_eth = TEST_AMOUNT_ETH
        config.token_decimals = 6  # USDC has 6 decimals
        config.slippage_tolerance = 0.5  # 0.5% slippage
        config.gas_limit = 300000
        config.gas_price_gwei = 100
        config.buyer_address = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689"  # Test address
        
        print("Starting simulation...")
        print("-" * 80)
        
        # Run the simulation
        print("🔄 Executing Buy → Approve → Sell sequence...")
        result = simulator.check_uniswap_v2_pool(
            token_address=USDC_ADDRESS,
            pool_address=USDC_WETH_V2_POOL,
            config=config
        )
        
        print()
        print("Transaction Results:")
        print("-" * 80)
        
        # Transaction success status
        print("\n📊 Transaction Status:")
        print(f"  1. Buy USDC:    {'✅ Success' if result.can_buy else '❌ Failed'}")
        print(f"  2. Approve:     {'✅ Success' if result.can_approve else '❌ Failed'}")
        print(f"  3. Sell USDC:   {'✅ Success' if result.can_sell else '❌ Failed'}")
        
        # Tax analysis
        print("\n💰 Tax Analysis:")
        print(f"  Buy Tax:  {result.buy_tax_percentage:.4f}%")
        print(f"  Sell Tax: {result.sell_tax_percentage:.4f}%")
        print(f"  Total Tax: {result.buy_tax_percentage + result.sell_tax_percentage:.4f}%")
        
        # Since we don't have direct access to token amounts in PyPoolBuySellSimulationResult,
        # we'll calculate expected amounts
        print("\n📈 Trade Flow Analysis:")
        print(f"  Block Number: {result.block_number}")
        print(f"  Pool Type: {result.pool_type}")
        
        # Calculate expected USDC amount (this is approximate)
        # At current prices, 1 ETH ≈ 3,800 USDC (this varies by block)
        eth_in_wei = int(TEST_AMOUNT_ETH * 1e18)
        
        print(f"\n  Input:")
        print(f"    • ETH Spent: {TEST_AMOUNT_ETH} ETH ({eth_in_wei} wei)")
        
        # Estimate USDC received (accounting for DEX fees and slippage)
        # Uniswap V2 has 0.3% fee, so we get 99.7% of the swap
        dex_fee = 0.003  # 0.3%
        effective_amount = TEST_AMOUNT_ETH * (1 - dex_fee)
        
        # Approximate ETH/USDC rate (this is just for demonstration)
        # In reality, this comes from the pool reserves
        approx_eth_price = 3800  # $3,800 per ETH (approximate)
        expected_usdc = effective_amount * approx_eth_price
        
        print(f"\n  Expected Output (before taxes):")
        print(f"    • USDC: ~{expected_usdc:,.2f} USDC")
        print(f"    • After {dex_fee*100:.1f}% DEX fee")
        
        if result.buy_tax_percentage > 0:
            after_tax_usdc = expected_usdc * (1 - result.buy_tax_percentage/100)
            print(f"    • After {result.buy_tax_percentage:.2f}% buy tax: ~{after_tax_usdc:,.2f} USDC")
        
        # Calculate round-trip loss
        print(f"\n💸 Round-Trip Cost Analysis:")
        total_dex_fees = dex_fee * 2 * 100  # Buy and sell fees
        total_taxes = result.buy_tax_percentage + result.sell_tax_percentage
        total_cost = total_dex_fees + total_taxes
        
        print(f"  DEX Fees (0.3% × 2):  {total_dex_fees:.2f}%")
        print(f"  Token Taxes:          {total_taxes:.2f}%")
        print(f"  Total Cost:           {total_cost:.2f}%")
        
        net_return = 100 - total_cost
        eth_returned = TEST_AMOUNT_ETH * (net_return / 100)
        eth_loss = TEST_AMOUNT_ETH - eth_returned
        
        print(f"\n  Net Return:  {net_return:.2f}% of initial ETH")
        print(f"  ETH Loss:    ~{eth_loss:.6f} ETH (${eth_loss * approx_eth_price:.2f} at ${approx_eth_price}/ETH)")
        
        # Summary
        print("\n" + "=" * 80)
        print("Summary")
        print("=" * 80)
        
        if result.can_buy and result.can_approve and result.can_sell:
            print("✅ USDC is fully tradeable on Uniswap V2")
            print(f"✅ Successfully simulated buy/approve/sell of {TEST_AMOUNT_ETH} ETH worth of USDC")
            
            if total_taxes == 0:
                print("✅ No token taxes detected (0% tax)")
                print(f"✅ Only DEX fees apply: ~{total_dex_fees:.2f}% round-trip")
            else:
                print(f"⚠️  Token has {total_taxes:.2f}% total tax")
                
            print(f"\n💡 Key Finding: A {TEST_AMOUNT_ETH} ETH round-trip trade would cost ~{total_cost:.2f}%")
            print(f"   You would receive back ~{eth_returned:.6f} ETH from your {TEST_AMOUNT_ETH} ETH")
            
        else:
            print("❌ Trading failed at some step")
            if result.error_message:
                print(f"   Error: {result.error_message}")
                
    except Exception as e:
        print(f"❌ Error running simulation: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()