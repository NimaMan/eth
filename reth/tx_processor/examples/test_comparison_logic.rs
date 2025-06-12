//! Test Comparison Logic
//! 
//! Validate the equality rules between Rust and Python state changes

use revm_tx_simulator_lib::process_tx::python_validator::{
    compare_state_changes, PythonProcessedTransaction, PythonEventCounts, PythonFees,
};
use revm_tx_simulator_lib::process_tx::{
    PythonCompatibleStateChanges, AddressStateChange, ProcessingMetadata,
};
use std::collections::HashMap;
use serde_json;

fn create_test_rust_changes(address: &str, eth_net: &str, tokens: Vec<(&str, &str)>) -> PythonCompatibleStateChanges {
    let mut state_changes = HashMap::new();
    let mut token_net = HashMap::new();
    
    for (token, amount) in tokens {
        token_net.insert(token.to_string(), amount.to_string());
    }
    
    state_changes.insert(address.to_string(), AddressStateChange {
        eth_net: eth_net.to_string(),
        token_net,
    });
    
    PythonCompatibleStateChanges {
        state_changes,
        metadata: ProcessingMetadata {
            tx_hash: "0x1234".to_string(),
            block_number: 18500000,
            processing_time_ms: 25.5,
            addresses_affected: 1,
            tokens_involved: tokens.len(),
        }
    }
}

fn create_test_python_tx(state_changes: HashMap<String, serde_json::Value>) -> Option<PythonProcessedTransaction> {
    Some(PythonProcessedTransaction {
        hash: "0x1234".to_string(),
        block_number: 18500000,
        block_timestamp: 1234567890,
        txn_index: 0,
        from_address: "0xabcd".to_string(),
        to_address: Some("0xefgh".to_string()),
        contract_address: None,
        value: 0,
        status: 1,
        nonce: 1,
        input: "0x".to_string(),
        txn_type: "transfer".to_string(),
        actions: vec!["transfer".to_string()],
        fees: Some(PythonFees {
            gas_price: 20_000_000_000,
            gas_used: 21000,
            txn_fee: 420_000_000_000_000,
        }),
        bribe_amount: None,
        unique_addresses: vec!["0xabcd".to_string(), "0xefgh".to_string()],
        erc20_contracts: vec![],
        event_counts: PythonEventCounts {
            erc20_transfers: 0,
            erc721_transfers: 0,
            erc1155_transfers: 0,
            internal_transactions: 0,
            uniswap_v2_swaps: 0,
            uniswap_v2_syncs: 0,
            uniswap_v3_swaps: 0,
            uniswap_v4_swaps: 0,
            approvals: 0,
            mints: 0,
            burns: 0,
            deposits: 0,
            withdraws: 0,
            permit2_events: 0,
            trading_enabled_events: 0,
            trading_disabled_events: 0,
        },
        erc20_transfers: vec![],
        internal_transactions: vec![],
        uniswap_v2_swaps: vec![],
        uniswap_v4_swaps: vec![],
        state_changes,
    })
}

fn test_perfect_match() {
    println!("🧪 Test 1: Perfect Match");
    
    let rust_changes = create_test_rust_changes(
        "0x1234",
        "1.5",
        vec![("USDC", "1000.0"), ("WETH", "0.5")]
    );

    let mut python_state_changes = HashMap::new();
    python_state_changes.insert("0x1234".to_string(), serde_json::json!({
        "eth_net": "1.5",
        "token_net": {
            "USDC": "1000.0",
            "WETH": "0.5"
        }
    }));

    let python_tx = create_test_python_tx(python_state_changes);
    let result = compare_state_changes(&rust_changes, &python_tx);

    if result.matches {
        println!("   ✅ PASS - Perfect match correctly identified");
    } else {
        println!("   ❌ FAIL - Perfect match not recognized");
        for diff in &result.differences {
            println!("      Difference: {} - {}", diff.field, diff.description);
        }
    }
}

fn test_zero_address_acceptable() {
    println!("🧪 Test 2: Zero Address Acceptable");
    
    let rust_changes = create_test_rust_changes(
        "0x5678",
        "0",
        vec![]
    );

    let python_state_changes = HashMap::new(); // Python omits zero-change address
    let python_tx = create_test_python_tx(python_state_changes);
    let result = compare_state_changes(&rust_changes, &python_tx);

    if result.matches {
        println!("   ✅ PASS - Zero address omission correctly accepted");
    } else {
        println!("   ❌ FAIL - Zero address omission incorrectly rejected");
        for diff in &result.differences {
            println!("      Difference: {} - {}", diff.field, diff.description);
        }
    }
}

fn test_formatting_differences_acceptable() {
    println!("🧪 Test 3: Formatting Differences Acceptable");
    
    let rust_changes = create_test_rust_changes(
        "0x1234",
        "1.500000000000000000",
        vec![("USDC", "1000.000000")]
    );

    let mut python_state_changes = HashMap::new();
    python_state_changes.insert("0x1234".to_string(), serde_json::json!({
        "eth_net": "1.5",
        "token_net": {
            "USDC": "1000"
        }
    }));

    let python_tx = create_test_python_tx(python_state_changes);
    let result = compare_state_changes(&rust_changes, &python_tx);

    if result.matches {
        println!("   ✅ PASS - Formatting differences correctly normalized");
    } else {
        println!("   ❌ FAIL - Formatting differences incorrectly rejected");
        for diff in &result.differences {
            println!("      Difference: {} - {}", diff.field, diff.description);
        }
    }
}

fn test_value_mismatch_error() {
    println!("🧪 Test 4: Value Mismatch Error");
    
    let rust_changes = create_test_rust_changes(
        "0x1234",
        "1.5",
        vec![("USDC", "1000")]
    );

    let mut python_state_changes = HashMap::new();
    python_state_changes.insert("0x1234".to_string(), serde_json::json!({
        "eth_net": "1.6", // Different value
        "token_net": {
            "USDC": "1000"
        }
    }));

    let python_tx = create_test_python_tx(python_state_changes);
    let result = compare_state_changes(&rust_changes, &python_tx);

    if !result.matches {
        println!("   ✅ PASS - Value mismatch correctly detected");
        
        // Check for ETH difference
        let eth_diff = result.differences.iter().find(|d| d.field.contains("eth_net"));
        if eth_diff.is_some() {
            println!("      ✅ ETH net difference correctly identified");
        } else {
            println!("      ❌ ETH net difference not found");
        }
    } else {
        println!("   ❌ FAIL - Value mismatch not detected");
    }
}

fn test_missing_token_error() {
    println!("🧪 Test 5: Missing Token Error");
    
    let rust_changes = create_test_rust_changes(
        "0x1234",
        "0",
        vec![("USDC", "1000"), ("WETH", "0.5")]
    );

    let mut python_state_changes = HashMap::new();
    python_state_changes.insert("0x1234".to_string(), serde_json::json!({
        "eth_net": "0",
        "token_net": {
            "USDC": "1000"
            // WETH missing
        }
    }));

    let python_tx = create_test_python_tx(python_state_changes);
    let result = compare_state_changes(&rust_changes, &python_tx);

    if !result.matches {
        println!("   ✅ PASS - Missing token correctly detected");
        
        // Check for WETH difference
        let weth_diff = result.differences.iter().find(|d| d.field.contains("WETH"));
        if weth_diff.is_some() {
            println!("      ✅ WETH difference correctly identified");
        } else {
            println!("      ❌ WETH difference not found");
        }
    } else {
        println!("   ❌ FAIL - Missing token not detected");
    }
}

fn test_zero_token_omission_acceptable() {
    println!("🧪 Test 6: Zero Token Omission Acceptable");
    
    let rust_changes = create_test_rust_changes(
        "0x1234",
        "1.5",
        vec![("USDC", "0"), ("WETH", "0.5")] // Zero USDC
    );

    let mut python_state_changes = HashMap::new();
    python_state_changes.insert("0x1234".to_string(), serde_json::json!({
        "eth_net": "1.5",
        "token_net": {
            "WETH": "0.5"
            // USDC omitted because it's zero
        }
    }));

    let python_tx = create_test_python_tx(python_state_changes);
    let result = compare_state_changes(&rust_changes, &python_tx);

    if result.matches {
        println!("   ✅ PASS - Zero token omission correctly accepted");
    } else {
        println!("   ❌ FAIL - Zero token omission incorrectly rejected");
        for diff in &result.differences {
            println!("      Difference: {} - {}", diff.field, diff.description);
        }
    }
}

fn main() {
    println!("🔍 Testing Python Integration Comparison Logic");
    println!("==============================================");
    println!();
    
    test_perfect_match();
    test_zero_address_acceptable();
    test_formatting_differences_acceptable();
    test_value_mismatch_error();
    test_missing_token_error();
    test_zero_token_omission_acceptable();
    
    println!();
    println!("🎯 Comparison Logic Test Summary");
    println!("   All tests validate the equality rules documented in PYTHON_INTEGRATION_SPEC.md");
    println!("   These rules ensure consistent state change comparison between Rust and Python");
    println!();
    println!("📚 Key Equality Rules Tested:");
    println!("   ✅ Addresses with zero changes: Rust shows \"0\", Python omits → EQUAL");
    println!("   ✅ Decimal formats: \"1.5\" vs \"1.500000\" → EQUAL");
    println!("   ✅ Zero tokens: Can be omitted from token_net → EQUAL");
    println!("   ❌ Different values: \"1.5\" vs \"1.6\" → NOT EQUAL");
    println!("   ❌ Missing non-zero amounts → NOT EQUAL");
}