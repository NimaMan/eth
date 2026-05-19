use std::str::FromStr;

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};
use eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};
use tx_simulator::{
    tx_builders::build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth,
    FullSimulationResult, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};

const DEFAULT_OWNER: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const DEFAULT_TOKEN: &str = "0x12512a46d971BD035D9466246bc46DAf389E281c";
const DEFAULT_SYMBOL: &str = "OTTO";
const DEFAULT_TOKEN_AMOUNT_RAW: &str = "100000000000000000000";
const DEFAULT_MIN_ETH_OUT_WEI: &str = "1";
const DEFAULT_DEADLINE: u64 = 1_800_000_000;

sol! {
    function balanceOf(address owner) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultSimulationRequest {
    pub owner: Option<String>,
    pub vault: Option<String>,
    pub token: Option<String>,
    pub symbol: Option<String>,
    pub transfer_amount_raw: Option<String>,
    pub sell_amount_raw: Option<String>,
    pub min_eth_out_wei: Option<String>,
    pub deadline: Option<u64>,
    pub block: Option<u64>,
    pub skip_transfer: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultSimulationReport {
    pub schema: &'static str,
    pub block: u64,
    pub owner: String,
    pub vault: String,
    pub token: String,
    pub symbol: String,
    pub deadline: u64,
    pub min_eth_out_wei: String,
    pub requested_transfer_amount_raw: String,
    pub balances_before: BalanceSnapshot,
    pub transfer: TransferSimulationReport,
    pub vault_sell: Option<VaultSellSimulationReport>,
    pub balances_after: BalanceSnapshot,
    pub chain_state: ChainStateReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceSnapshot {
    pub owner_token_raw: String,
    pub vault_token_raw: String,
    pub owner_eth_wei: String,
    pub vault_eth_wei: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferSimulationReport {
    pub skipped: bool,
    pub success: Option<bool>,
    pub gas_used: Option<u64>,
    pub logs: Option<usize>,
    pub revert_reason: Option<String>,
    pub vault_received_raw: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultSellSimulationReport {
    pub success: bool,
    pub gas_used: u64,
    pub logs: usize,
    pub revert_reason: Option<String>,
    pub sell_amount_raw: String,
    pub selector: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainStateReport {
    pub transactions_executed: usize,
    pub total_gas_used: u64,
}

pub async fn simulate_uniswap_v2_trading_vault_wallet_token(
    simulator: &TxSimulator,
    request: VaultSimulationRequest,
) -> Result<VaultSimulationReport> {
    let owner = parse_address(request_text(&request.owner, DEFAULT_OWNER), "owner")?;
    let vault = parse_address(request_text(&request.vault, DEFAULT_VAULT), "vault")?;
    let token = parse_address(request_text(&request.token, DEFAULT_TOKEN), "token")?;
    let transfer_amount = parse_u256(
        request_text(&request.transfer_amount_raw, DEFAULT_TOKEN_AMOUNT_RAW),
        "transfer_amount_raw",
    )?;
    let requested_sell_amount = parse_optional_u256(&request.sell_amount_raw, "sell_amount_raw")?;
    let min_eth_out = parse_u256(
        request_text(&request.min_eth_out_wei, DEFAULT_MIN_ETH_OUT_WEI),
        "min_eth_out_wei",
    )?;
    let deadline = request.deadline.unwrap_or(DEFAULT_DEADLINE);
    let symbol = request
        .symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SYMBOL)
        .to_string();
    let skip_transfer = request.skip_transfer.unwrap_or(false);

    let block = request
        .block
        .unwrap_or(simulator.latest_historical_context_block_number()?);
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;

    let owner_token_before = erc20_balance(&mut chain, token, owner)?;
    let vault_token_before = erc20_balance(&mut chain, token, vault)?;
    let owner_eth_before = chain.eth_balance(owner)?;
    let vault_eth_before = chain.eth_balance(vault)?;

    if !skip_transfer && owner_token_before < transfer_amount {
        return Err(eyre!(
            "owner balance {} is below requested transfer amount {}",
            owner_token_before,
            transfer_amount
        ));
    }

    let transfer_result = if skip_transfer {
        None
    } else {
        Some(
            chain
                .step_with_trace(erc20_transfer_tx(owner, token, vault, transfer_amount))
                .await
                .wrap_err("token transfer simulation failed")?,
        )
    };

    let vault_token_after_transfer = erc20_balance(&mut chain, token, vault)?;
    let vault_received = vault_token_after_transfer.saturating_sub(vault_token_before);
    let sell_amount = requested_sell_amount.unwrap_or_else(|| {
        if skip_transfer {
            transfer_amount
        } else {
            vault_received
        }
    });

    let sell_result = if skip_transfer
        || transfer_result
            .as_ref()
            .map(|result| result.success)
            .unwrap_or(false)
    {
        Some(
            chain
                .step_with_trace(
                    build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
                        vault,
                        owner,
                        token,
                        sell_amount,
                        min_eth_out,
                        deadline,
                    ),
                )
                .await
                .wrap_err("vault emergency sell simulation failed")?,
        )
    } else {
        None
    };

    let owner_token_after = erc20_balance(&mut chain, token, owner)?;
    let vault_token_after = erc20_balance(&mut chain, token, vault)?;
    let owner_eth_after = chain.eth_balance(owner)?;
    let vault_eth_after = chain.eth_balance(vault)?;
    let state = chain.current_state();

    Ok(VaultSimulationReport {
        schema: "uniswap_v2_trading_vault_wallet_token_simulation_v1",
        block,
        owner: owner.to_string(),
        vault: vault.to_string(),
        token: token.to_string(),
        symbol,
        deadline,
        min_eth_out_wei: min_eth_out.to_string(),
        requested_transfer_amount_raw: transfer_amount.to_string(),
        balances_before: BalanceSnapshot {
            owner_token_raw: owner_token_before.to_string(),
            vault_token_raw: vault_token_before.to_string(),
            owner_eth_wei: owner_eth_before.to_string(),
            vault_eth_wei: vault_eth_before.to_string(),
        },
        transfer: transfer_report(skip_transfer, transfer_result.as_ref(), vault_received),
        vault_sell: sell_result
            .as_ref()
            .map(|result| sell_report(result, sell_amount)),
        balances_after: BalanceSnapshot {
            owner_token_raw: owner_token_after.to_string(),
            vault_token_raw: vault_token_after.to_string(),
            owner_eth_wei: owner_eth_after.to_string(),
            vault_eth_wei: vault_eth_after.to_string(),
        },
        chain_state: ChainStateReport {
            transactions_executed: state.transaction_count,
            total_gas_used: state.total_gas_used,
        },
    })
}

fn transfer_report(
    skipped: bool,
    result: Option<&FullSimulationResult>,
    vault_received: U256,
) -> TransferSimulationReport {
    TransferSimulationReport {
        skipped,
        success: result.map(|result| result.success),
        gas_used: result.map(|result| result.gas_used),
        logs: result.map(|result| result.logs.len()),
        revert_reason: result.and_then(|result| result.revert_reason.clone()),
        vault_received_raw: vault_received.to_string(),
    }
}

fn sell_report(result: &FullSimulationResult, sell_amount: U256) -> VaultSellSimulationReport {
    VaultSellSimulationReport {
        success: result.success,
        gas_used: result.gas_used,
        logs: result.logs.len(),
        revert_reason: result.revert_reason.clone(),
        sell_amount_raw: sell_amount.to_string(),
        selector: "0x5f413d10",
    }
}

fn request_text<'a>(value: &'a Option<String>, default: &'static str) -> &'a str {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
}

fn parse_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value).wrap_err_with(|| format!("invalid {label} address {value:?}"))
}

fn parse_u256(value: &str, label: &str) -> Result<U256> {
    U256::from_str(value).wrap_err_with(|| format!("invalid {label} value {value:?}"))
}

fn parse_optional_u256(value: &Option<String>, label: &str) -> Result<Option<U256>> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| parse_u256(value, label))
        .transpose()
}

fn erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
) -> Result<U256> {
    let data = Bytes::from(balanceOfCall { owner }.abi_encode());
    let result = chain.simulate_view_call(token, data)?;
    if !result.success {
        return Err(eyre!("balanceOf({owner}) failed for token {token}"));
    }
    decode_u256(&result.output, "balanceOf output")
}

fn erc20_transfer_tx(
    owner: Address,
    token: Address,
    recipient: Address,
    amount: U256,
) -> UnsignedTransaction {
    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(
            transferCall {
                to: recipient,
                amount,
            }
            .abi_encode(),
        )),
        nonce: None,
        ..Default::default()
    }
}

fn decode_u256(output: &Bytes, label: &str) -> Result<U256> {
    if output.len() < 32 {
        return Err(eyre!("{label} is shorter than 32 bytes"));
    }
    Ok(U256::from_be_slice(&output[..32]))
}
