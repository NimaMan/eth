/*
* Transaction Executor
*
* Responsible for:
* - Transaction signing using private keys
* - Nonce management
* - Transaction submission to the network
* - Gas price strategies
* - Retry mechanisms
*
* Algorithm:
* 1. Sign transaction with appropriate wallet
* 2. Determine nonce (track locally to avoid RPC calls)
* 3. Set gas price based on transaction priority
* 4. Submit transaction to network
* 5. Handle retries for failed submissions
* 6. Return transaction hash
*/

use crate::tx_execution::{
    types::{Transaction, TxStatus, TxReceipt},
    config::TxExecutionConfig,
    error::{TxError, TxResult},
};
use ethers::{
    providers::Middleware,
    types::{Address, U256, Bytes, H256, TransactionRequest},
    signers::Signer,
    middleware::SignerMiddleware,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::SystemTime;

/// Transaction executor for signing and submitting transactions
pub struct TxExecutor<S, M> 
where
    S: Signer + Clone,
    M: Middleware + Clone
{
    /// Provider for blockchain interaction
    provider: Arc<M>,
    
    /// Configuration for transaction execution
    config: TxExecutionConfig,
    
    /// Wallet for signing
    signer: S,
    
    /// Nonce tracker (address -> next nonce)
    nonces: Mutex<HashMap<Address, U256>>,
}

impl<S, M> TxExecutor<S, M> 
where
    S: Signer + Clone,
    M: Middleware + Clone
{
    /// Create a new transaction executor
    pub fn new(provider: Arc<M>, signer: S, config: TxExecutionConfig) -> Self {
        Self {
            provider,
            config,
            signer,
            nonces: Mutex::new(HashMap::new()),
        }
    }
    
    /// Execute a transaction
    pub async fn execute(&self, tx: &Transaction) -> TxResult<TxStatus> {
        // Verify the from address matches our signer
        if tx.params.from != self.signer.address() {
            return Err(TxError::SigningFailed(format!(
                "Transaction from address ({:?}) does not match signer address ({:?})",
                tx.params.from, self.signer.address()
            )));
        }
        
        // Get nonce
        let nonce = self.get_nonce(tx.params.from).await?;
        
        // Create a request
        let mut request = self.create_tx_request(tx, nonce);
        
        // Apply gas price strategy based on urgency
        self.apply_gas_strategy(&mut request, tx.urgent);
        
        // Create a signer middleware for signing and sending
        let client = SignerMiddleware::new(self.provider.clone(), self.signer.clone());
        
        // Sign and send the transaction
        let pending_tx = client
            .send_transaction(request, None)
            .await
            .map_err(|e| TxError::SubmissionFailed(format!("Failed to submit transaction: {}", e)))?;
        
        // Get the transaction hash
        let tx_hash = pending_tx.tx_hash();
        
        // Update nonce
        self.increment_nonce(tx.params.from);
        
        // Return initial status
        Ok(TxStatus::Submitted {
            time: SystemTime::now(),
            hash: tx_hash,
        })
    }
    
    /// Get current nonce for an address, either from cache or RPC
    async fn get_nonce(&self, address: Address) -> TxResult<U256> {
        let mut nonces = self.nonces.lock().unwrap();
        
        if let Some(nonce) = nonces.get(&address) {
            return Ok(*nonce);
        }
        
        // Query from provider
        let nonce = self.provider
            .get_transaction_count(address, None)
            .await
            .map_err(|e| TxError::ProviderError(format!("Failed to get nonce: {}", e)))?;
        
        // Cache the nonce
        nonces.insert(address, nonce);
        
        Ok(nonce)
    }
    
    /// Increment nonce for an address
    fn increment_nonce(&self, address: Address) {
        let mut nonces = self.nonces.lock().unwrap();
        
        if let Some(nonce) = nonces.get_mut(&address) {
            *nonce = *nonce + U256::one();
        }
    }
    
    /// Create a transaction request
    fn create_tx_request(&self, tx: &Transaction, nonce: U256) -> TransactionRequest {
        let mut request = TransactionRequest::new()
            .from(tx.params.from)
            .data(tx.params.data.clone())
            .value(tx.params.value)
            .nonce(nonce);
        
        // Add to address if not contract creation
        if let Some(to) = tx.params.to {
            request = request.to(to);
        }
        
        // Add gas limit
        request = request.gas(tx.params.gas_limit);
        
        // Set chain ID
        request = request.chain_id(tx.params.chain_id);
        
        request
    }
    
    /// Apply gas price strategy based on urgency
    fn apply_gas_strategy(&self, request: &mut TransactionRequest, urgent: bool) {
        // For legacy transactions
        let mut gas_price = self.config.base_gas_price;
        
        if urgent {
            // Add urgency boost
            let boost = gas_price * U256::from(self.config.urgent_gas_boost_percent) / U256::from(100);
            gas_price = gas_price + boost;
        }
        
        // Set gas price (for both legacy and EIP-1559, ethers-rs handles the conversion)
        request.gas_price = Some(gas_price);
    }
    
    /// Get transaction receipt
    pub async fn get_receipt(&self, tx_hash: H256) -> TxResult<Option<TxReceipt>> {
        match self.provider.get_transaction_receipt(tx_hash).await {
            Ok(Some(receipt)) => Ok(Some(receipt.into())),
            Ok(None) => Ok(None),
            Err(err) => Err(TxError::ProviderError(format!("Failed to get receipt: {}", err))),
        }
    }
}
