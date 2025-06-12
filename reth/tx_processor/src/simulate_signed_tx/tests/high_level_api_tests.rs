//! Tests for the high-level simulate_signed_tx API
//! These tests verify the async functions that fetch data from RPC

#[cfg(test)]
mod tests {
    use crate::simulate_signed_tx::{simulate_signed_tx, simulate_signed_tx_bytes};
    use crate::simulate_signed_tx::simulation_core::ExecutionResultType;
    use ethers_core::types::H256;
    use revm_context::result::SuccessReason;
    use std::str::FromStr;
    use anyhow::Result;

    const RPC_URL: &str = "http://127.0.0.1:8545";

    #[tokio::test]
    async fn test_simulate_signed_tx_by_hash() -> Result<()> {
        // Use a well-known mainnet transaction
        let tx_hash = H256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
        
        // Simulate the transaction
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        // Verify results
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ), "Transaction should succeed");
        
        assert_eq!(output.gas_used, 315099, "Gas usage should match known value");
        assert_eq!(output.gas_refunded, 78774, "Gas refund should match");
        assert_eq!(output.logs.len(), 13, "Should have 13 logs");
        assert!(output.output_data.is_empty(), "No output data for this transaction");
        
        // Internal transfers not yet implemented
        assert!(output.internal_transfers.is_empty(), "Internal transfers TODO");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_not_found() {
        // Use a non-existent transaction hash
        let tx_hash = H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
        
        // Should fail
        let result = simulate_signed_tx(tx_hash, RPC_URL).await;
        
        assert!(result.is_err(), "Should fail for non-existent transaction");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"), "Error should mention transaction not found");
    }

    #[tokio::test]
    async fn test_unmined_transaction() {
        // This would require creating a pending transaction, which is complex
        // For now, we'll skip this test but document it
        // TODO: Test with a transaction in mempool
    }

    #[tokio::test]
    async fn test_simulate_signed_tx_bytes() -> Result<()> {
        // First, get a real transaction to extract its bytes
        let tx_hash = H256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
        
        // We need to fetch the transaction to get its raw bytes
        use ethers_providers::{Provider, Http, Middleware};
        use std::sync::Arc;
        
        let provider = Provider::<Http>::try_from(RPC_URL)?;
        let client = Arc::new(provider);
        
        let tx = client.get_transaction(tx_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
        
        // Serialize to RLP bytes
        use ethers_core::utils::rlp;
        let tx_bytes = rlp::encode(&tx);
        
        // Now simulate using bytes
        let block_number = tx.block_number.unwrap().as_u64();
        let output = simulate_signed_tx_bytes(&tx_bytes, block_number, RPC_URL).await?;
        
        // Should get same results as simulating by hash
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ));
        assert_eq!(output.gas_used, 315099);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_different_rpc_endpoints() {
        // Test with invalid RPC endpoint
        let tx_hash = H256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae").unwrap();
        let invalid_rpc = "http://invalid.endpoint:8545";
        
        let result = simulate_signed_tx(tx_hash, invalid_rpc).await;
        assert!(result.is_err(), "Should fail with invalid RPC");
    }

    #[tokio::test]
    async fn test_error_propagation() -> Result<()> {
        // Test that errors are properly propagated with context
        let tx_hash = H256::from_str("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")?;
        
        match simulate_signed_tx(tx_hash, RPC_URL).await {
            Ok(_) => panic!("Should have failed"),
            Err(e) => {
                // Verify error contains useful context
                let error_str = e.to_string();
                assert!(
                    error_str.contains("Transaction not found") || 
                    error_str.contains("not found"),
                    "Error should be descriptive: {}",
                    error_str
                );
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_simple_eth_transfer() -> Result<()> {
        // Test with a known simple ETH transfer
        // Block 46147 - one of the earliest transactions
        let tx_hash = H256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060")?;
        
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        // Simple transfer properties
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ));
        assert_eq!(output.gas_used, 21000, "Simple transfer uses exactly 21k gas");
        assert_eq!(output.gas_refunded, 0, "No refund for simple transfer");
        assert!(output.logs.is_empty(), "No logs for simple ETH transfer");
        assert!(output.output_data.is_empty(), "No output for simple transfer");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_erc20_transfer() -> Result<()> {
        // Test with known ERC20 transfer
        let tx_hash = H256::from_str("0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b")?;
        
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ));
        
        // ERC20 transfers should have logs (Transfer event)
        assert!(!output.logs.is_empty(), "ERC20 transfer should emit Transfer event");
        
        // Check for Transfer event topic
        let transfer_topic = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        let has_transfer = output.logs.iter().any(|log| {
            log.topics().first()
                .map(|t| format!("{:x}", t) == transfer_topic)
                .unwrap_or(false)
        });
        assert!(has_transfer, "Should have Transfer event");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_simulations() -> Result<()> {
        // Test that multiple simulations can run concurrently
        use tokio::task::JoinSet;
        
        let tx_hashes = vec![
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
            "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae",
            "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b",
        ];
        
        let mut join_set = JoinSet::new();
        
        for hash_str in tx_hashes {
            let hash = H256::from_str(hash_str).unwrap();
            join_set.spawn(simulate_signed_tx(hash, RPC_URL));
        }
        
        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            results.push(result.unwrap());
        }
        
        // All should succeed
        for (i, result) in results.into_iter().enumerate() {
            assert!(result.is_ok(), "Simulation {} should succeed", i);
            let output = result.unwrap();
            assert!(matches!(
                output.result_type,
                ExecutionResultType::Success(_)
            ));
        }
        
        Ok(())
    }
}