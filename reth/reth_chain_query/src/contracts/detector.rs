use alloy_primitives::Address;
use eyre::Result;
use reth_primitives::Bytes;

use crate::utils::function_signatures::FUNCTION_SIGNATURES;
use crate::provider::RethQueryProvider;

const REQUIRED_SELECTORS: [&str; 6] = [
    "18160ddd",
    "70a08231",
    "a9059cbb",
    "23b872dd",
    "095ea7b3",
    "dd62ed3e",
];

impl RethQueryProvider {
    pub async fn is_erc20_contract(&self, address: Address, block_number: Option<u64>) -> Result<bool> {
        let code = self.get_runtime_code(address, block_number).await?;
        Ok(REQUIRED_SELECTORS.iter().all(|sig| selector_in_code(sig, &code)))
    }

    async fn get_runtime_code(&self, address: Address, block_number: Option<u64>) -> Result<Bytes> {
        let provider = self.provider_factory.provider()?;
        let code = provider
            .code_by_hash(address, block_number.unwrap_or(self.get_latest_block()?))?
            .unwrap_or_default();
        Ok(Bytes::from(code))
    }
}

fn selector_in_code(selector_hex: &str, code: &Bytes) -> bool {
    let sig_bytes = hex::decode(selector_hex).expect("valid hex selector");
    code.windows(4).any(|window| window == sig_bytes.as_slice())
}
