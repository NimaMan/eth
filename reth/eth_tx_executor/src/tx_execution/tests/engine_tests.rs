#[cfg(test)]
mod engine_tests {
    use crate::tx_execution::{
        engine::TxExecutionEngine,
        types::{Transaction, TransactionParams, TransactionType, TxStatus, SimulationResult, StateChange, StateChangeType, SimulationError},
        config::TxExecutionConfig,
        error::TxResult,
    };
    use ethers::{
        providers::{Provider, Http, Middleware, PendingTransaction},
        types::{Address, U256, Bytes, H256, TransactionReceipt, BlockNumber, TransactionRequest},
        signers::{Signer},
        utils::parse_ether,
    };
    use std::sync::Arc;
    use std::str::FromStr;
    use mockall::predicate::*;
    use mockall::mock;

    // Mock the Provider for controlled testing
    mock! {
        pub Provider<Http> {
            fn call(&self, tx: &ethers::types::TransactionRequest, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<Bytes, ethers::providers::ProviderError>> + Send>>;
            
            fn estimate_gas(&self, tx: &ethers::types::TransactionRequest, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U256, ethers::providers::ProviderError>> + Send>>;
            
            fn get_balance(&self, address: Address, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U256, ethers::providers::ProviderError>> + Send>>;
                
            fn get_transaction_count(&self, address: Address, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U256, ethers::providers::ProviderError>> + Send>>;
                
            fn get_transaction_receipt(&self, tx_hash: H256) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<TransactionReceipt>, ethers::providers::ProviderError>> + Send>>;
                
            fn get_block_number(&self) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<U64, ethers::providers::ProviderError>> + Send>>;
                
            fn send_transaction<T: Into<TransactionRequest> + Send + Sync>(&self, tx: T, block: Option<ethers::types::BlockId>) -> 
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<PendingTransaction<'static, Provider<Http>>, ethers::providers::ProviderError>> + Send>>;
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
    async fn test_engine_successful_execution_flow() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Common setup
        let wallet = MockWallet::new(TEST_WALLET);
        let config = setup_test_config();
        let tx = create_test_transaction();
        
        // Set up mock behaviors - Simulation
        // For call
        mock_provider
            .expect_call()
            .returning(|_, _| Box::pin(std::future::ready(Ok(Bytes::from(vec![1])))));
            
        // For gas estimation
        mock_provider
            .expect_estimate_gas()
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(50_000)))));
            
        // For balance checks
        mock_provider
            .expect_get_balance()
            .returning(|_, _| Box::pin(std::future::ready(Ok(parse_ether("10.0").unwrap()))));
            
        // For nonce
        mock_provider
            .expect_get_transaction_count()
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(42)))));
            
        // For transaction submission
        mock_provider
            .expect_send_transaction()
            .returning(move |_, _| {
                let pending = PendingTransaction::new(tx_hash, Arc::new(MockProvider::new()));
                Box::pin(std::future::ready(Ok(pending)))
            });
        
        // Create the engine
        let provider = Arc::new(mock_provider);
        let engine = TxExecutionEngine::new(provider, wallet, config);
        
        // Execute the transaction
        let result = engine.execute(&tx).await.unwrap();
        
        // Verify the result
        match result {
            TxStatus::Submitted { hash, .. } => {
                assert_eq!(hash, tx_hash);
            },
            _ => panic!("Expected Submitted status, got {:?}", result),
        }
    }
    
    #[tokio::test]
    async fn test_engine_simulation_only() {
        let mut mock_provider = MockProvider::new();
        
        // Common setup
        let wallet = MockWallet::new(TEST_WALLET);
        let config = setup_test_config();
        let tx = create_test_transaction();
        
        // Set up mock behaviors - Simulation
        // For call
        mock_provider
            .expect_call()
            .returning(|_, _| Box::pin(std::future::ready(Ok(Bytes::from(vec![1])))));
            
        // For gas estimation
        mock_provider
            .expect_estimate_gas()
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(50_000)))));
            
        // For balance checks
        mock_provider
            .expect_get_balance()
            .returning(|_, _| Box::pin(std::future::ready(Ok(parse_ether("10.0").unwrap()))));
        
        // Create the engine
        let provider = Arc::new(mock_provider);
        let engine = TxExecutionEngine::new(provider, wallet, config);
        
        // Only simulate the transaction
        let simulation = engine.simulate_only(&tx).await.unwrap();
        
        // Verify simulation results
        assert!(simulation.success);
        assert_eq!(simulation.gas_used, U256::from(50_000));
        assert!(simulation.result.is_some());
    }
    
    #[tokio::test]
    async fn test_engine_simulation_failure() {
        let mut mock_provider = MockProvider::new();
        
        // Common setup
        let wallet = MockWallet::new(TEST_WALLET);
        let config = setup_test_config();
        let tx = create_test_transaction();
        
        // Set up mock behaviors - Failed simulation
        // For call - revert
        mock_provider
            .expect_call()
            .returning(|_, _| {
                let err = ethers::providers::ProviderError::JsonRpcClientError(
                    ethers::providers::JsonRpcError::Call(String::from("execution reverted"))
                );
                Box::pin(std::future::ready(Err(err)))
            });
            
        // Create the engine
        let provider = Arc::new(mock_provider);
        let engine = TxExecutionEngine::new(provider, wallet, config);
        
        // Execute the transaction - should fail at simulation stage
        let result = engine.execute(&tx).await;
        
        // Verify the execution fails with simulation error
        assert!(result.is_err());
        match result {
            Err(err) => {
                assert!(format!("{}", err).contains("Simulation failed"));
            },
            _ => panic!("Expected simulation error"),
        }
    }
    
    #[tokio::test]
    async fn test_engine_gas_limit_exceeded() {
        let mut mock_provider = MockProvider::new();
        
        // Common setup
        let wallet = MockWallet::new(TEST_WALLET);
        let config = setup_test_config();
        
        // Create a transaction with low gas limit
        let mut tx = create_test_transaction();
        tx.params.gas_limit = U256::from(21_000); // Low gas limit
        
        // Set up mock behaviors
        // For call - successful
        mock_provider
            .expect_call()
            .returning(|_, _| Box::pin(std::future::ready(Ok(Bytes::from(vec![1])))));
            
        // For gas estimation - returns high value exceeding limit
        mock_provider
            .expect_estimate_gas()
            .returning(|_, _| Box::pin(std::future::ready(Ok(U256::from(100_000)))));
            
        // For balance checks
        mock_provider
            .expect_get_balance()
            .returning(|_, _| Box::pin(std::future::ready(Ok(parse_ether("10.0").unwrap()))));
        
        // Create the engine
        let provider = Arc::new(mock_provider);
        let engine = TxExecutionEngine::new(provider, wallet, config);
        
        // Execute the transaction - should fail at validation stage
        let result = engine.execute(&tx).await;
        
        // Verify the execution fails with gas error
        assert!(result.is_err());
        match result {
            Err(err) => {
                assert!(format!("{}", err).contains("Gas required"));
            },
            _ => panic!("Expected gas limit error"),
        }
    }
    
    #[tokio::test]
    async fn test_engine_get_status() {
        let mut mock_provider = MockProvider::new();
        let tx_hash = H256::random();
        
        // Block numbers
        let block_num = 15_000_000u64;
        let latest_block = block_num + 5; // 5 confirmations
        
        // Set up mock receipt - transaction is confirmed
        let receipt = create_mock_receipt(tx_hash, block_num, true);
        
        mock_provider
            .expect_get_transaction_receipt()
            .returning(move |hash| {
                assert_eq!(hash, tx_hash);
                Box::pin(std::future::ready(Ok(Some(receipt.clone()))))
            });
            
        // Set up latest block number
        mock_provider
            .expect_get_block_number()
            .returning(move |_| Box::pin(std::future::ready(Ok(latest_block.into()))));
        
        // Common setup
        let wallet = MockWallet::new(TEST_WALLET);
        let config = setup_test_config();
        
        // Create the engine
        let provider = Arc::new(mock_provider);
        let engine = TxExecutionEngine::new(provider, wallet, config);
        
        // Get status
        let status = engine.get_status(tx_hash).await.unwrap();
        
        // Verify confirmed status
        match status {
            TxStatus::Confirmed { hash, confirmations, .. } => {
                assert_eq!(hash, tx_hash);
                assert_eq!(confirmations, 5);
            },
            _ => panic!("Expected Confirmed status, got {:?}", status),
        }
    }
} 