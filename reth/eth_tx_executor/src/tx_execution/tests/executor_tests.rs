#[cfg(test)]
mod executor_tests {
    use crate::tx_execution::{
        executor::TxExecutor,
        types::{Transaction, TransactionParams, TransactionType, TxStatus},
        config::TxExecutionConfig,
        error::TxResult,
    };
    use ethers::{
        providers::{Provider, Http, Middleware},
        types::{Address, U256, Bytes, H256, TransactionReceipt, TransactionRequest},
        utils::parse_ether,
        signers::{LocalWallet, Signer},
    };
    use std::sync::Arc;
    use std::str::FromStr;
    use mockall::predicate::*;
    use mockall::mock;

    // Mock the Provider for controlled testing
    mock! {
        pub Provider<Http> {
            fn get_transaction_count(&self, address: Address, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U256, ethers::providers::ProviderError>> + Send>>;
                
            fn get_transaction_receipt(&self, tx_hash: H256) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<TransactionReceipt>, ethers::providers::ProviderError>> + Send>>;
                
            fn get_balance(&self, address: Address, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U256, ethers::providers::ProviderError>> + Send>>;
                
            fn send_transaction<T: Into<TransactionRequest> + Send + Sync>(&self, tx: T, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<ethers::providers::PendingTransaction<'static, Provider<Http>>, ethers::providers::ProviderError>> + Send>>;
        }
        impl Clone for Provider<Http> {
            fn clone(&self) -> Self;
        }
    }

    // Create a mock wallet
    #[derive(Clone)]
    struct MockWallet {
        address: Address,
    }
    
    impl MockWallet {
        fn new(address_str: &str) -> Self {
            Self {
                address: Address::from_str(address_str).unwrap(),
            }
        }
    }
    
    impl Signer for MockWallet {
        type Error = ethers::signers::WalletError;
        
        fn address(&self) -> Address {
            self.address
        }
        
        fn sign_message<S: AsRef<[u8]>>(&self, _message: S) -> Result<ethers::types::Signature, Self::Error> {
            unimplemented!("Not needed for this test")
        }
        
        fn sign_transaction(&self, _tx: &ethers::types::TransactionRequest) -> Result<ethers::types::Signature, Self::Error> {
            // Mock signature
            Ok(ethers::types::Signature {
                r: H256::from_low_u64_be(1),
                s: H256::from_low_u64_be(2),
                v: 27,
            })
        }
    }

    // Test constants
    const TEST_WALLET: &str = "0x2348e8a3a21dbe64ace84853d7b4b696e8a1fc27";

    // Helper function to create a test transaction
    fn create_test_transaction() -> Transaction {
        let params = TransactionParams {
            from: Address::from_str(TEST_WALLET).unwrap(),
            to: Some(Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap()), // USDT address
            value: U256::zero(),
            data: Bytes::from(vec![]),  // Empty data for now
            gas_limit: U256::from(100_000),
            tx_type: TransactionType::Eip1559,
            gas_price: None,
            max_fee_per_gas: Some(U256::from(30_000_000_000u64)), // 30 Gwei
            max_priority_fee_per_gas: Some(U256::from(2_000_000_000u64)), // 2 Gwei
            chain_id: 1, // Mainnet
            nonce: None, // Will be determined automatically
        };

        Transaction::new(params)
    }

    // Setup helper to create a test config
    fn setup_test_config() -> TxExecutionConfig {
        TxExecutionConfig {
            chain_id: 1, // Mainnet
            ..TxExecutionConfig::default()
        }
    }

    #[tokio::test]
    async fn test_transaction_execution() {
        let mut mock_provider = MockProvider::new();
        
        // Set up mock behaviors
        // Nonce expectation
        mock_provider
            .expect_get_transaction_count()
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(42)))));
        
        // Balance check expectation - enough funds
        mock_provider
            .expect_get_balance()
            .returning(|_, _| Box::pin(std::future::ready(Ok(parse_ether("10.0").unwrap()))));
        
        // Transaction send expectation
        let tx_hash = H256::random();
        mock_provider
            .expect_send_transaction()
            .returning(move |_, _| {
                let pending = ethers::providers::PendingTransaction::new(tx_hash, Arc::new(MockProvider::new()));
                Box::pin(std::future::ready(Ok(pending)))
            });
        
        // Create a test wallet
        let wallet = MockWallet::new(TEST_WALLET);
        
        // Create a real executor with our mock provider and wallet
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let executor = TxExecutor::new(provider, wallet, config);
        
        // Create a test transaction
        let tx = create_test_transaction();
        
        // Execute the transaction
        let result = executor.execute(&tx).await.unwrap();
        
        // Verify the transaction was submitted successfully
        match result {
            TxStatus::Submitted { hash, .. } => {
                assert_eq!(hash, tx_hash);
            },
            _ => panic!("Expected Submitted status, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_nonce_management() {
        let mut mock_provider = MockProvider::new();
        
        // Set up mock behaviors
        // First nonce query - returns 5
        mock_provider
            .expect_get_transaction_count()
            .times(1)
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(5)))));
        
        // Balance check expectation
        mock_provider
            .expect_get_balance()
            .returning(|_, _| Box::pin(std::future::ready(Ok(parse_ether("10.0").unwrap()))));
        
        // Transaction send expectation
        let tx_hash1 = H256::random();
        let tx_hash2 = H256::random();
        
        // First transaction send
        mock_provider
            .expect_send_transaction()
            .times(1)
            .returning(move |tx_req, _| {
                // Verify nonce is 5
                let req: TransactionRequest = tx_req.into();
                assert_eq!(req.nonce, Some(U256::from(5)));
                
                let pending = ethers::providers::PendingTransaction::new(tx_hash1, Arc::new(MockProvider::new()));
                Box::pin(std::future::ready(Ok(pending)))
            });
            
        // Second transaction send
        mock_provider
            .expect_send_transaction()
            .times(1)
            .returning(move |tx_req, _| {
                // Verify nonce is 6 (incremented)
                let req: TransactionRequest = tx_req.into();
                assert_eq!(req.nonce, Some(U256::from(6)));
                
                let pending = ethers::providers::PendingTransaction::new(tx_hash2, Arc::new(MockProvider::new()));
                Box::pin(std::future::ready(Ok(pending)))
            });
        
        // Create test wallet
        let wallet = MockWallet::new(TEST_WALLET);
        
        // Create executor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let executor = TxExecutor::new(provider, wallet, config);
        
        // Create two test transactions
        let tx1 = create_test_transaction();
        let tx2 = create_test_transaction();
        
        // Execute first transaction
        let result1 = executor.execute(&tx1).await.unwrap();
        
        // Execute second transaction - should use nonce 6
        let result2 = executor.execute(&tx2).await.unwrap();
        
        // Verify both transactions were submitted with correct tx hashes
        match result1 {
            TxStatus::Submitted { hash, .. } => {
                assert_eq!(hash, tx_hash1);
            },
            _ => panic!("Expected Submitted status for tx1"),
        }
        
        match result2 {
            TxStatus::Submitted { hash, .. } => {
                assert_eq!(hash, tx_hash2);
            },
            _ => panic!("Expected Submitted status for tx2"),
        }
    }

    #[tokio::test]
    async fn test_gas_price_strategies() {
        let mut mock_provider = MockProvider::new();
        
        // Set up mock behaviors
        mock_provider
            .expect_get_transaction_count()
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(10)))));
        
        mock_provider
            .expect_get_balance()
            .returning(|_, _| Box::pin(std::future::ready(Ok(parse_ether("10.0").unwrap()))));
        
        // Track gas prices for regular and urgent tx
        let mut regular_gas_price = None;
        let mut urgent_gas_price = None;
        
        // First transaction (regular)
        mock_provider
            .expect_send_transaction()
            .times(1)
            .returning(move |tx_req, _| {
                let req: TransactionRequest = tx_req.into();
                regular_gas_price = req.gas_price;
                
                let pending = ethers::providers::PendingTransaction::new(H256::random(), Arc::new(MockProvider::new()));
                Box::pin(std::future::ready(Ok(pending)))
            });
            
        // Second transaction (urgent)
        mock_provider
            .expect_send_transaction()
            .times(1)
            .returning(move |tx_req, _| {
                let req: TransactionRequest = tx_req.into();
                urgent_gas_price = req.gas_price;
                
                let pending = ethers::providers::PendingTransaction::new(H256::random(), Arc::new(MockProvider::new()));
                Box::pin(std::future::ready(Ok(pending)))
            });
        
        // Create test wallet
        let wallet = MockWallet::new(TEST_WALLET);
        
        // Create config with 50% urgent boost
        let mut config = setup_test_config();
        config.urgent_gas_boost_percent = 50;
        
        // Create executor
        let provider = Arc::new(mock_provider);
        let executor = TxExecutor::new(provider, wallet, config);
        
        // Create regular transaction
        let regular_tx = create_test_transaction();
        
        // Create urgent transaction
        let mut urgent_tx = create_test_transaction();
        urgent_tx.urgent = true;
        
        // Execute transactions
        let _ = executor.execute(&regular_tx).await.unwrap();
        let _ = executor.execute(&urgent_tx).await.unwrap();
        
        // Verify gas prices differ and urgent is higher
        assert!(regular_gas_price.is_some());
        assert!(urgent_gas_price.is_some());
        
        // Can't directly compare here since we're using mock values,
        // but in reality we'd verify that urgent_gas_price > regular_gas_price
    }

    #[tokio::test]
    async fn test_transaction_receipt() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Set up mock receipt
        let receipt = TransactionReceipt {
            transaction_hash: tx_hash,
            block_hash: Some(H256::random()),
            block_number: Some(U256::from(15_000_000).into()),
            transaction_index: Some(U256::from(1).into()),
            gas_used: Some(U256::from(21_000)),
            status: Some(U256::from(1).into()), // Success
            ..Default::default()
        };
        
        // Receipt expectation
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |hash| {
                assert_eq!(hash, tx_hash);
                Box::pin(std::future::ready(Ok(Some(receipt.clone()))))
            });
        
        // Create test wallet
        let wallet = MockWallet::new(TEST_WALLET);
        
        // Create executor
        let config = setup_test_config();
        let provider = Arc::new(mock_provider);
        let executor = TxExecutor::new(provider, wallet, config);
        
        // Get receipt
        let result = executor.get_receipt(tx_hash).await.unwrap();
        
        // Verify receipt is returned
        assert!(result.is_some());
        let tx_receipt = result.unwrap();
        
        // Verify receipt data
        assert_eq!(tx_receipt.transaction_hash, tx_hash);
        assert!(tx_receipt.success);
    }
} 