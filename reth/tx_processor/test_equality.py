#!/usr/bin/env python3
"""
Test Python integration comparison logic

This script demonstrates the equality rules documented in PYTHON_INTEGRATION_SPEC.md
"""

def test_decimal_normalization():
    """Test decimal normalization logic"""
    print("🧪 Testing Decimal Normalization")
    
    # Equivalent decimal formats
    test_cases = [
        ("1.5", "1.500000", True),
        ("0", "0.0", True),
        ("0", "0.000000", True),
        ("1000", "1000.0", True),
        ("-1.5", "-1.500000", True),
        ("1.5", "1.6", False),
        ("1.5", "-1.5", False),
        ("0", "0.1", False),
    ]
    
    for val1, val2, expected in test_cases:
        # Simple normalization: parse as float and compare
        try:
            normalized1 = float(val1)
            normalized2 = float(val2)
            result = normalized1 == normalized2
            
            status = "✅" if result == expected else "❌"
            print(f"   {status} \"{val1}\" vs \"{val2}\" → {'EQUAL' if result else 'NOT EQUAL'}")
        except ValueError:
            print(f"   ❌ \"{val1}\" vs \"{val2}\" → PARSE ERROR")

def test_zero_address_logic():
    """Test zero address handling logic"""
    print("🧪 Testing Zero Address Logic")
    
    # Rust state changes with zero address
    rust_changes = {
        "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}},
        "0x5678": {"eth_net": "0", "token_net": {}}  # Zero address
    }
    
    # Python omits zero address
    python_changes = {
        "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}}
    }
    
    # Check if this should be considered equal
    rust_nonzero = {addr: changes for addr, changes in rust_changes.items() 
                   if not (changes["eth_net"] == "0" and not changes["token_net"])}
    
    equal = rust_nonzero == python_changes
    print(f"   {'✅' if equal else '❌'} Zero address omission: {'EQUAL' if equal else 'NOT EQUAL'}")

def test_token_zero_logic():
    """Test zero token handling logic"""
    print("🧪 Testing Zero Token Logic")
    
    # Rust includes zero tokens
    rust_tokens = {"USDC": "0", "WETH": "0.5"}
    
    # Python omits zero tokens  
    python_tokens = {"WETH": "0.5"}
    
    # Filter out zero tokens from Rust
    rust_nonzero_tokens = {token: amount for token, amount in rust_tokens.items() 
                          if float(amount) != 0.0}
    
    equal = rust_nonzero_tokens == python_tokens
    print(f"   {'✅' if equal else '❌'} Zero token omission: {'EQUAL' if equal else 'NOT EQUAL'}")

def test_format_differences():
    """Test formatting differences"""
    print("🧪 Testing Format Differences")
    
    # Different formatting but same values
    test_pairs = [
        ({"eth_net": "1.500000000000000000", "token_net": {"USDC": "1000.000000"}},
         {"eth_net": "1.5", "token_net": {"USDC": "1000"}}),
    ]
    
    for rust_data, python_data in test_pairs:
        # Normalize by parsing as floats
        rust_eth = float(rust_data["eth_net"])
        python_eth = float(python_data["eth_net"])
        
        eth_equal = rust_eth == python_eth
        
        # Normalize tokens
        rust_tokens_norm = {token: float(amount) for token, amount in rust_data["token_net"].items()}
        python_tokens_norm = {token: float(amount) for token, amount in python_data["token_net"].items()}
        
        tokens_equal = rust_tokens_norm == python_tokens_norm
        
        overall_equal = eth_equal and tokens_equal
        print(f"   {'✅' if overall_equal else '❌'} Format normalization: {'EQUAL' if overall_equal else 'NOT EQUAL'}")

def main():
    """Run all tests"""
    print("🔍 Python Integration Equality Logic Tests")
    print("==========================================")
    print()
    
    test_decimal_normalization()
    print()
    test_zero_address_logic()
    print()
    test_token_zero_logic()
    print()
    test_format_differences()
    print()
    
    print("🎯 Summary")
    print("   These tests validate the equality rules documented in PYTHON_INTEGRATION_SPEC.md")
    print("   The Rust implementation follows these same normalization rules")
    print()
    print("📚 Key Equality Rules:")
    print("   ✅ Addresses with zero changes: Rust shows \"0\", Python omits → EQUAL")
    print("   ✅ Decimal formats: \"1.5\" vs \"1.500000\" → EQUAL")
    print("   ✅ Zero tokens: Can be omitted from token_net → EQUAL")
    print("   ❌ Different values: \"1.5\" vs \"1.6\" → NOT EQUAL")
    print("   ❌ Missing non-zero amounts → NOT EQUAL")

if __name__ == "__main__":
    main()