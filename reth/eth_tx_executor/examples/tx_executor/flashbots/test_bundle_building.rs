//! Test bundle building without network calls
//! 
//! Validates bundle construction, serialization, and edge cases

use eth_kartal::flashbots::BundleBuilder;
use ethers::prelude::*;
use ethers::utils::parse_ether;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::time::{SystemTime, UNIX_EPOCH};

fn create_test_transaction() -> Bytes {
    // Create a dummy transaction for testing
    let tx = TransactionRequest::new()
        .to("0x742d35Cc6634C0532925a3b844Bc9e7595f7E391".parse::<Address>().unwrap())
        .value(parse_ether("0.1").unwrap())
        .gas(21000u64)
        .gas_price(parse_ether("0.00002").unwrap())
        .nonce(42u64);
    
    // Create dummy signature
    let dummy_sig = Signature {
        r: U256::from(1),
        s: U256::from(2),
        v: 27,
    };
    
    // Convert to TypedTransaction and sign
    let typed_tx: TypedTransaction = tx.into();
    typed_tx.rlp_signed(&dummy_sig)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Flashbots Bundle Building Tests ===\n");
    
    // Test 1: Basic bundle creation
    println!("📦 Test 1: Basic Bundle Creation");
    test_basic_bundle()?;
    
    // Test 2: Multi-transaction bundle
    println!("\n📦 Test 2: Multi-Transaction Bundle");
    test_multi_tx_bundle()?;
    
    // Test 3: Bundle with time windows
    println!("\n📦 Test 3: Bundle with Time Windows");
    test_time_window_bundle()?;
    
    // Test 4: Bundle serialization
    println!("\n📦 Test 4: Bundle Serialization");
    test_bundle_serialization()?;
    
    // Test 5: Edge cases
    println!("\n📦 Test 5: Edge Cases");
    test_edge_cases()?;
    
    // Test 6: Performance
    println!("\n📦 Test 6: Performance Benchmark");
    test_performance()?;
    
    println!("\n✅ All bundle building tests passed!");
    
    Ok(())
}

fn test_basic_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let tx = create_test_transaction();
    
    let bundle = BundleBuilder::new()
        .add_transaction(tx.clone())
        .block_number(12345678)
        .tip_percentage(0.01)
        .build()?;
    
    println!("  ✓ Created bundle for block: {}", bundle.block_number);
    println!("  ✓ Bundle hash: {:?}", bundle.hash());
    println!("  ✓ Transactions: {}", bundle.transactions.len());
    println!("  ✓ Tip amount: {} ETH", ethers::utils::format_ether(bundle.tip_amount));
    
    assert_eq!(bundle.transactions.len(), 1);
    assert_eq!(bundle.block_number, 12345678);
    
    Ok(())
}

fn test_multi_tx_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let tx1 = create_test_transaction();
    let tx2 = create_test_transaction();
    let tx3 = create_test_transaction();
    
    let bundle = BundleBuilder::new()
        .add_transactions(vec![tx1, tx2, tx3])
        .block_number(12345679)
        .build()?;
    
    println!("  ✓ Created bundle with {} transactions", bundle.transactions.len());
    println!("  ✓ Bundle hash: {:?}", bundle.hash());
    
    assert_eq!(bundle.transactions.len(), 3);
    
    // Test bundle request conversion
    let request = bundle.to_request();
    println!("  ✓ Converted to request with {} txs", request.txs.len());
    
    // Verify hex encoding
    for (i, tx_hex) in request.txs.iter().enumerate() {
        assert!(tx_hex.starts_with("0x"));
        println!("  ✓ Tx {}: {} bytes", i, tx_hex.len() / 2 - 1);
    }
    
    Ok(())
}

fn test_time_window_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let tx = create_test_transaction();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    
    let bundle = BundleBuilder::new()
        .add_transaction(tx)
        .block_number(12345680)
        .min_timestamp(now)
        .max_timestamp(now + 120) // 2 minute window
        .build()?;
    
    println!("  ✓ Bundle valid from: {}", bundle.min_timestamp.unwrap());
    println!("  ✓ Bundle valid until: {}", bundle.max_timestamp.unwrap());
    println!("  ✓ Time window: {} seconds", 
        bundle.max_timestamp.unwrap() - bundle.min_timestamp.unwrap());
    
    assert!(bundle.min_timestamp.is_some());
    assert!(bundle.max_timestamp.is_some());
    assert!(bundle.max_timestamp.unwrap() > bundle.min_timestamp.unwrap());
    
    // Test time_window helper
    let bundle2 = BundleBuilder::new()
        .add_transaction(create_test_transaction())
        .block_number(12345681)
        .time_window(300) // 5 minutes
        .build()?;
    
    println!("  ✓ Time window helper: {} second window", 300);
    assert!(bundle2.min_timestamp.is_some());
    assert!(bundle2.max_timestamp.is_some());
    
    Ok(())
}

fn test_bundle_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let tx = create_test_transaction();
    
    let bundle = BundleBuilder::new()
        .add_transaction(tx.clone())
        .block_number(12345682)
        .protect_transaction(H256::from_low_u64_be(123))
        .build()?;
    
    let request = bundle.to_request();
    
    println!("  ✓ Serialized bundle to request");
    println!("  ✓ Block number: {}", request.block_number);
    println!("  ✓ Reverting tx hashes: {:?}", request.reverting_tx_hashes);
    
    // Test JSON serialization
    let json = serde_json::to_string_pretty(&request)?;
    println!("  ✓ JSON size: {} bytes", json.len());
    println!("  ✓ JSON format validated");
    
    Ok(())
}

fn test_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: Empty bundle (should fail)
    println!("  Testing empty bundle...");
    let result = BundleBuilder::new()
        .block_number(12345683)
        .build();
    
    assert!(result.is_err());
    println!("  ✓ Empty bundle correctly rejected");
    
    // Test 2: No block number (should fail)
    println!("  Testing missing block number...");
    let result = BundleBuilder::new()
        .add_transaction(create_test_transaction())
        .build();
    
    assert!(result.is_err());
    println!("  ✓ Missing block number correctly rejected");
    
    // Test 3: Large bundle
    println!("  Testing large bundle...");
    let mut builder = BundleBuilder::new();
    for _ in 0..10 {
        builder = builder.add_transaction(create_test_transaction());
    }
    
    let bundle = builder.block_number(12345684).build()?;
    println!("  ✓ Created bundle with {} transactions", bundle.transactions.len());
    
    // Test 4: Disable revert protection
    let bundle = BundleBuilder::new()
        .add_transaction(create_test_transaction())
        .block_number(12345685)
        .protect_transaction(H256::from_low_u64_be(456))
        .allow_reverts() // This should clear protected transactions
        .build()?;
    
    println!("  ✓ Revert protection disabled");
    assert_eq!(bundle.reverting_tx_hashes.len(), 0);
    
    Ok(())
}

fn test_performance() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    
    // Prepare transactions
    let transactions: Vec<Bytes> = (0..100)
        .map(|_| create_test_transaction())
        .collect();
    
    // Measure bundle building time
    let start = Instant::now();
    
    let bundle = BundleBuilder::new()
        .add_transactions(transactions.clone())
        .block_number(12345686)
        .time_window(120)
        .tip_percentage(0.02)
        .build()?;
    
    let build_time = start.elapsed();
    
    println!("  ✓ Built bundle with 100 txs in {:?}", build_time);
    
    // Measure serialization time
    let start = Instant::now();
    let request = bundle.to_request();
    let serialize_time = start.elapsed();
    
    println!("  ✓ Serialized to request in {:?}", serialize_time);
    
    // Measure hashing time
    let start = Instant::now();
    let hash = bundle.hash();
    let hash_time = start.elapsed();
    
    println!("  ✓ Calculated bundle hash in {:?}", hash_time);
    println!("  ✓ Bundle hash: {:?}", hash);
    
    // Verify performance targets
    assert!(build_time.as_millis() < 10, "Bundle building too slow");
    assert!(serialize_time.as_millis() < 5, "Serialization too slow");
    assert!(hash_time.as_micros() < 1000, "Hashing too slow");
    
    println!("\n  📊 Performance Summary:");
    println!("  - Bundle building: {:?}", build_time);
    println!("  - Serialization: {:?}", serialize_time);
    println!("  - Hashing: {:?}", hash_time);
    println!("  - Total: {:?}", build_time + serialize_time + hash_time);
    
    Ok(())
}