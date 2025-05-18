#[cfg(test)]
mod monitor_tests {
    use crate::tx_execution::{
        monitor::TxMonitor,
        types::{TxStatus, TxConfirmation},
        config::TxExecutionConfig,
        error::TxResult,
    };
    use ethers::{
        providers::{Provider, Http, Middleware},
        types::{Address, U256, H256, TransactionReceipt, BlockNumber},
    };
    use std::sync::Arc;
    use std::time::Duration;
    use mockall::predicate::*;
    use mockall::mock;

    // Mock the Provider for controlled testing
    mock! {
        pub Provider<Http> {
            fn get_transaction_receipt(&self, tx_hash: H256) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<TransactionReceipt>, ethers::providers::ProviderError>> + Send>>;
                
            fn get_block_number(&self) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U64, ethers::providers::ProviderError>> + Send>>;
        }
        impl Clone for Provider<Http> {
            fn clone(&self) -> Self;
        }
    }

    // Setup helper to create a test config
    fn setup_test_config() -> TxExecutionConfig {
        TxExecutionConfig {
            chain_id: 1, // Mainnet
            default_confirmation_blocks: 3,
            ..TxExecutionConfig::default()
        }
    }
    
    // Helper to create a mock receipt
    fn create_mock_receipt(tx_hash: H256, block_number: u64, success: bool) -> TransactionReceipt {
        TransactionReceipt {
            transaction_hash: tx_hash,
            block_hash: Some(H256::random()),
            block_number: Some(BlockNumber::from(block_number)),
            transaction_index: Some(U256::from(0).into()),
            gas_used: Some(U256::from(21_000)),
            status: Some(U256::from(if success { 1 } else { 0 }).into()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_transaction_pending_status() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Set up mock behaviors - no receipt means pending
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |_| Box::pin(std::future::ready(Ok(None))));
        
        // Create monitor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let monitor = TxMonitor::new(provider, config);
        
        // Get status
        let status = monitor.get_transaction_status(tx_hash).await.unwrap();
        
        // Verify pending status
        match status {
            TxStatus::Pending { hash, .. } => {
                assert_eq!(hash, tx_hash);
            },
            _ => panic!("Expected Pending status, got {:?}", status),
        }
    }
    
    #[tokio::test]
    async fn test_transaction_mined_status() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Block numbers
        let block_num = 15_000_000u64;
        let latest_block = block_num + 1; // Just 1 confirmation
        
        // Set up mock receipt - transaction is mined
        let receipt = create_mock_receipt(tx_hash, block_num, true);
        
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |_| Box::pin(std::future::ready(Ok(Some(receipt.clone())))));
            
        // Set up latest block number
        mock_provider
            .expect_get_block_number()
            .returning(move |_| Box::pin(std::future::ready(Ok(latest_block.into()))));
        
        // Create monitor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let monitor = TxMonitor::new(provider, config);
        
        // Get status
        let status = monitor.get_transaction_status(tx_hash).await.unwrap();
        
        // Verify mined status (not yet confirmed with config requiring 3 confirmations)
        match status {
            TxStatus::Mined { hash, block_number, .. } => {
                assert_eq!(hash, tx_hash);
                assert_eq!(block_number, U256::from(block_num));
            },
            _ => panic!("Expected Mined status, got {:?}", status),
        }
    }
    
    #[tokio::test]
    async fn test_transaction_confirmed_status() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Block numbers
        let block_num = 15_000_000u64;
        let latest_block = block_num + 5; // 5 confirmations
        
        // Set up mock receipt - transaction is mined
        let receipt = create_mock_receipt(tx_hash, block_num, true);
        
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |_| Box::pin(std::future::ready(Ok(Some(receipt.clone())))));
            
        // Set up latest block number
        mock_provider
            .expect_get_block_number()
            .returning(move |_| Box::pin(std::future::ready(Ok(latest_block.into()))));
        
        // Create monitor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let monitor = TxMonitor::new(provider, config.clone());
        
        // Get status
        let status = monitor.get_transaction_status(tx_hash).await.unwrap();
        
        // Verify confirmed status (5 confirmations > config's 3 required)
        match status {
            TxStatus::Confirmed { hash, confirmations, .. } => {
                assert_eq!(hash, tx_hash);
                assert_eq!(confirmations, 5);
            },
            _ => panic!("Expected Confirmed status, got {:?}", status),
        }
    }
    
    #[tokio::test]
    async fn test_confirmation_info() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Block numbers
        let block_num = 15_000_000u64;
        let latest_block = block_num + 2; // 2 confirmations
        
        // Set up mock receipt
        let receipt = create_mock_receipt(tx_hash, block_num, true);
        
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |_| Box::pin(std::future::ready(Ok(Some(receipt.clone())))));
            
        // Set up latest block number
        mock_provider
            .expect_get_block_number()
            .returning(move |_| Box::pin(std::future::ready(Ok(latest_block.into()))));
        
        // Create monitor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let monitor = TxMonitor::new(provider, config);
        
        // Get confirmation info with 3 required confirmations
        let confirmation_info = monitor.get_confirmation_info(tx_hash, 3).await.unwrap();
        
        // Verify confirmation info
        assert_eq!(confirmation_info.hash, tx_hash);
        assert_eq!(confirmation_info.confirmations, 2);
        assert_eq!(confirmation_info.required_confirmations, 3);
        assert_eq!(confirmation_info.is_confirmed, false); // 2 < 3 required
        
        // Verify block numbers
        assert_eq!(confirmation_info.block_number, U256::from(block_num));
        assert_eq!(confirmation_info.current_block, U256::from(latest_block));
    }
    
    #[tokio::test]
    async fn test_failed_transaction() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Block numbers
        let block_num = 15_000_000u64;
        
        // Set up mock receipt - transaction failed (status 0)
        let receipt = create_mock_receipt(tx_hash, block_num, false);
        
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |_| Box::pin(std::future::ready(Ok(Some(receipt.clone())))));
            
        // Set up latest block number (not needed for failed tx)
        mock_provider
            .expect_get_block_number()
            .returning(|_| Box::pin(std::future::ready(Ok((block_num + 1).into()))));
        
        // Create monitor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let monitor = TxMonitor::new(provider, config);
        
        // Get status
        let status = monitor.get_transaction_status(tx_hash).await.unwrap();
        
        // For a failed transaction, our monitor should return Failed status
        match status {
            TxStatus::Failed { hash, .. } => {
                assert_eq!(hash, tx_hash);
            },
            _ => panic!("Expected Failed status, got {:?}", status),
        }
    }
} 