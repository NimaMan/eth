/// Test Function Signature Detection for Liquidity Removal
/// 
/// This example tests the detection of liquidity removal transactions
/// by checking their function signatures in the input data.
///
/// Usage:
///    cargo run --example test_function_signature_detection --release

use hex;
use tracing::{info, warn};

/// Check if transaction is a liquidity removal based on function signature
fn is_liquidity_removal(input_data: &Option<Vec<u8>>) -> Option<&'static str> {
    if let Some(data) = input_data {
        if data.len() >= 4 {
            let selector = hex::encode(&data[0..4]);
            match selector.as_str() {
                "02751cec" => Some("removeLiquidityETH"),
                "baa2abde" => Some("removeLiquidity"),
                "af2979eb" => Some("removeLiquidityETHSupportingFeeOnTransferTokens"),
                "5b0d5984" => Some("removeLiquidityETHWithPermit"),
                "ded9382a" => Some("removeLiquidityETHWithPermitSupportingFeeOnTransferTokens"),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("Testing Function Signature Detection");
    info!("===================================");
    
    // Test cases with real function signatures
    let test_cases = vec![
        // removeLiquidityETH
        ("0x02751cec", "removeLiquidityETH"),
        // removeLiquidity
        ("0xbaa2abde", "removeLiquidity"),
        // removeLiquidityETHSupportingFeeOnTransferTokens
        ("0xaf2979eb", "removeLiquidityETHSupportingFeeOnTransferTokens"),
        // removeLiquidityETHWithPermit
        ("0x5b0d5984", "removeLiquidityETHWithPermit"),
        // removeLiquidityETHWithPermitSupportingFeeOnTransferTokens
        ("0xded9382a", "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens"),
        // Some non-liquidity removal functions for comparison
        ("0x095ea7b3", "approve (not liquidity removal)"),
        ("0xa9059cbb", "transfer (not liquidity removal)"),
        ("0x18160ddd", "totalSupply (not liquidity removal)"),
    ];
    
    info!("\n1. Testing known function signatures:");
    for (hex_sig, expected_name) in &test_cases {
        let data = hex::decode(hex_sig.strip_prefix("0x").unwrap_or(hex_sig)).unwrap();
        let input_data = Some(data);
        
        match is_liquidity_removal(&input_data) {
            Some(detected_name) => {
                info!("✅ {} detected as: {}", hex_sig, detected_name);
                if expected_name.contains("not liquidity removal") {
                    warn!("   WARNING: This should NOT be detected as liquidity removal!");
                }
            }
            None => {
                if expected_name.contains("not liquidity removal") {
                    info!("✅ {} correctly NOT detected as liquidity removal", hex_sig);
                } else {
                    warn!("❌ {} NOT detected (expected: {})", hex_sig, expected_name);
                }
            }
        }
    }
    
    // Test with actual transaction data from the example
    info!("\n2. Testing with real transaction data:");
    
    // This is a removeLiquidityETH transaction
    let real_tx_data = "0x02751cec00000000000000000000000000c30e4da69c8e91b8d9f156d850f597cbe674adb5000000000000000000000000000000000000000000001152a971a99c10e875300000000000000000000000000000000000000000000000127eb0fc2ab2b8506c000000000000000000000000000000000000000000000000001f9c0b8e1f46350000000000000000000000008a25b20d6c83c98f4d2bcdf86ef24ccd97e8c17300000000000000000000000000000000000000000000000000000000676dc86b";
    
    let data = hex::decode(real_tx_data.strip_prefix("0x").unwrap_or(real_tx_data)).unwrap();
    let input_data = Some(data.clone());
    
    info!("Transaction input data length: {} bytes", data.len());
    info!("First 4 bytes (function selector): 0x{}", hex::encode(&data[0..4]));
    
    match is_liquidity_removal(&input_data) {
        Some(function_name) => {
            info!("✅ Detected as liquidity removal: {}", function_name);
            
            // Decode the parameters (for removeLiquidityETH)
            if data.len() >= 196 {  // 4 + 6*32 bytes
                info!("\nDecoded parameters:");
                info!("  Token: 0x{}", hex::encode(&data[16..36]));
                info!("  Liquidity: 0x{}", hex::encode(&data[36..68]));
                info!("  AmountTokenMin: 0x{}", hex::encode(&data[68..100]));
                info!("  AmountETHMin: 0x{}", hex::encode(&data[100..132]));
                info!("  To: 0x{}", hex::encode(&data[144..164]));
                info!("  Deadline: 0x{}", hex::encode(&data[164..196]));
            }
        }
        None => {
            warn!("❌ NOT detected as liquidity removal!");
        }
    }
    
    // Test edge cases
    info!("\n3. Testing edge cases:");
    
    // Empty data
    let empty_data: Option<Vec<u8>> = None;
    match is_liquidity_removal(&empty_data) {
        Some(_) => warn!("❌ Empty data detected as liquidity removal!"),
        None => info!("✅ Empty data correctly NOT detected"),
    }
    
    // Too short data
    let short_data = Some(vec![0x02, 0x75]);  // Only 2 bytes
    match is_liquidity_removal(&short_data) {
        Some(_) => warn!("❌ Short data detected as liquidity removal!"),
        None => info!("✅ Short data correctly NOT detected"),
    }
    
    // Unknown function signature
    let unknown_data = Some(vec![0x12, 0x34, 0x56, 0x78]);
    match is_liquidity_removal(&unknown_data) {
        Some(_) => warn!("❌ Unknown signature detected as liquidity removal!"),
        None => info!("✅ Unknown signature correctly NOT detected"),
    }
    
    info!("\n=== Summary ===");
    info!("The function signature detection correctly identifies liquidity removal transactions");
    info!("based on their 4-byte function selectors at the beginning of the input data.");
}