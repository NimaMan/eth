//! Run examples for simulate_signed_tx module
//! This allows running examples that are within the component

#[cfg(test)]
mod example_runs {
    // Example runner utilities
    use anyhow::Result;
    
    #[tokio::test]
    async fn run_basic_simulation() -> Result<()> {
        println!("Running basic_simulation example from component");
        
        // This runs the actual simulation
        use ethers_core::types::H256 as EthersH256;
        use ethers_providers::{Middleware, Provider as EthersProvider, Http as EthersHttp};
        use std::sync::Arc;
        
        let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
        let tx_hash_parsed: EthersH256 = tx_hash.parse()?;
        
        let ethers_provider = EthersProvider::<EthersHttp>::try_from("http://127.0.0.1:8545")?;
        let eth_client = Arc::new(ethers_provider);
        
        let tx = eth_client.get_transaction(tx_hash_parsed).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
        
        println!("✅ Successfully fetched transaction: {}", tx_hash);
        println!("   From: {:?}", tx.from);
        println!("   To: {:?}", tx.to);
        println!("   Gas: {}", tx.gas);
        
        Ok(())
    }
    
    #[test]
    fn run_call_tracer_example() {
        println!("Running call_tracer_usage example from component");
        
        use crate::simulate_signed_tx::CallTracer;
        use revm_primitives::{Address, U256};
        
        let mut tracer = CallTracer::new();
        
        // Record some transfers
        tracer.record_internal_transfer(
            Address::from([1u8; 20]),
            Address::from([2u8; 20]),
            U256::from(1_000_000_000_000_000_000u128)
        );
        
        let transfers = tracer.get_internal_transfers();
        println!("✅ CallTracer captured {} transfers", transfers.len());
        
        assert_eq!(transfers.len(), 1);
    }
}