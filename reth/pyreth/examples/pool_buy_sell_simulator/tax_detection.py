#!/usr/bin/env python3
"""
Tax Detection Example

Demonstrates how to detect token taxes using the pool_buy_sell_simulator.
This example uses RFI (Reflect Finance) which has a known 1% fee on all transfers.

The simulator calculates taxes by comparing expected vs actual token amounts
received during buy and sell transactions.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import pyreth


def analyze_token_with_tax(simulator, token_name: str, token_addr: str, pool_addr: str, 
                          expected_tax: float, decimals: int = 18):
    """Analyze a token known to have taxes"""
    print(f"\nAnalyzing {token_name} Token")
    print("=" * 50)
    print(f"Token Address: {token_addr}")
    print(f"Pool Address:  {pool_addr}")
    print(f"Expected Tax:  {expected_tax}% on all transfers")
    print(f"Decimals:      {decimals}")
    print()
    
    try:
        # Create custom config with specific parameters
        config = pyreth.PoolViabilityConfig()
        config.test_amount_eth = 1.0  # Use 1 ETH for better tax detection
        config.token_decimals = decimals
        config.block_delay = 1  # Sell in next block to avoid MEV
        
        print(f"Running simulation with {config.test_amount_eth} ETH...")
        
        # Check the pool
        result = simulator.check_uniswap_v2_pool(
            token_address=token_addr,
            pool_address=pool_addr,
            config=config
        )
        
        # Display results
        print("\nResults:")
        print("--------")
        print(f"Can Buy:     {'✅' if result.can_buy else '❌'}")
        print(f"Can Approve: {'✅' if result.can_approve else '❌'}")
        print(f"Can Sell:    {'✅' if result.can_sell else '❌'}")
        print()
        
        # Tax analysis
        print("Tax Detection:")
        print(f"  Detected Buy Tax:  {result.buy_tax_percentage:.2f}%")
        print(f"  Detected Sell Tax: {result.sell_tax_percentage:.2f}%")
        print(f"  Total Tax:         {result.buy_tax_percentage + result.sell_tax_percentage:.2f}%")
        print()
        
        # Verify against expected
        buy_diff = abs(result.buy_tax_percentage - expected_tax)
        sell_diff = abs(result.sell_tax_percentage - expected_tax)
        
        if buy_diff < 0.5:  # Within 0.5% tolerance
            print(f"✅ Buy tax matches expected ({expected_tax}% ± 0.5%)")
        else:
            print(f"⚠️  Buy tax differs from expected by {buy_diff:.2f}%")
            
        if sell_diff < 0.5:
            print(f"✅ Sell tax matches expected ({expected_tax}% ± 0.5%)")
        else:
            print(f"⚠️  Sell tax differs from expected by {sell_diff:.2f}%")
            
        return result
        
    except Exception as e:
        print(f"❌ Error analyzing {token_name}: {e}")
        return None


def main():
    print("=" * 70)
    print("Token Tax Detection Using Pool Buy Sell Simulator")
    print("=" * 70)
    print()
    print("This example demonstrates tax detection by testing tokens")
    print("with known transfer fees/taxes.")
    print()
    
    try:
        # Initialize
        reth = pyreth.PyReth()
        simulator = reth.pool_buy_sell_simulator()
        print("✅ Pool Buy Sell Simulator initialized")
        
        # Test tokens with known taxes
        test_cases = [
            # (name, token_address, pool_address, expected_tax%, decimals)
            ("RFI", 
             "0xa1afffe3f4d611d252010e3eaf6f4d77088b0cd7",
             "0xb9ca9f213667ffd221f078ecf3a72dafe04d45ab", 
             1.0, 9),  # RFI has 1% fee
             
            ("BABYSHIBA",
             "0x0198BE93B7cae38b0005fb8c77D5B56965304Fa4",
             "0xa0c3298e973ae0e7f49d8dc957c6ad203e0a8d4c",
             5.0, 9),  # BABYSHIBA typically has higher taxes
        ]
        
        # Analyze each token
        for name, token, pool, expected_tax, decimals in test_cases:
            result = analyze_token_with_tax(
                simulator, name, token, pool, expected_tax, decimals
            )
            
            if result and result.error_message:
                print(f"\n⚠️  Additional info: {result.error_message}")
                
        # Summary
        print("\n" + "=" * 70)
        print("Tax Detection Summary")
        print("=" * 70)
        print("\nThe pool_buy_sell_simulator successfully detects token taxes by:")
        print("1. Simulating a buy transaction and measuring tokens received")
        print("2. Comparing actual vs expected amounts to calculate buy tax")
        print("3. Simulating a sell transaction and measuring ETH received")
        print("4. Comparing actual vs expected amounts to calculate sell tax")
        print("\nThis is useful for:")
        print("• Identifying high-tax tokens before trading")
        print("• Calculating accurate profit/loss including taxes")
        print("• Filtering out tokens with excessive fees")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()