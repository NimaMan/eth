use std::str::FromStr;

use alloy_primitives::{hex, keccak256, Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};
use clap::{Parser, ValueEnum};
use eyre::{eyre, Result, WrapErr};
use serde_json::{json, Value};
use tx_simulator::{
    tx_builders::{
        build_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens,
        build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth,
    },
    FullSimulationResult, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};

const DEFAULT_OWNER: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const DEFAULT_TOKEN: &str = "0x12512a46d971BD035D9466246bc46DAf389E281c";
const DEFAULT_SYMBOL: &str = "OTTO";
const DEFAULT_TRANSFER_AMOUNT_RAW: &str = "100000000000000000000";
const DEFAULT_BUY_ETH_WEI: &str = "10000000000000000";
const DEFAULT_MIN_OUT: &str = "1";
const DEFAULT_DEADLINE: u64 = 1_800_000_000;
const DEFAULT_MAX_FEE_PER_GAS_WEI: &str = "50000000000";
const DEFAULT_PRIORITY_FEE_PER_GAS_WEI: &str = "2000000000";

sol! {
    function balanceOf(address owner) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
}

#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum Scenario {
    WalletTransferThenSell,
    BuyThenSell,
    CurrentStateSell,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum TxPosition {
    Before,
    After,
}

#[derive(Debug, Parser)]
#[command(about = "Simulate the deployed Uniswap V2 trading vault against local Reth state")]
struct Args {
    #[arg(long, default_value = DEFAULT_OWNER)]
    owner: String,
    #[arg(long, default_value = DEFAULT_VAULT)]
    vault: String,
    #[arg(long, default_value = DEFAULT_TOKEN)]
    token: String,
    #[arg(long, default_value = DEFAULT_SYMBOL)]
    symbol: String,
    #[arg(long, value_enum, default_value_t = Scenario::WalletTransferThenSell)]
    scenario: Scenario,
    #[arg(long, default_value = DEFAULT_TRANSFER_AMOUNT_RAW)]
    transfer_amount_raw: String,
    #[arg(long)]
    sell_amount_raw: Option<String>,
    #[arg(long, default_value = DEFAULT_BUY_ETH_WEI)]
    buy_eth_wei: String,
    #[arg(long, default_value = DEFAULT_MIN_OUT)]
    min_tokens_out_raw: String,
    #[arg(long, default_value = DEFAULT_MIN_OUT)]
    min_eth_out_wei: String,
    #[arg(long, default_value_t = DEFAULT_DEADLINE)]
    deadline: u64,
    #[arg(long)]
    block: Option<u64>,
    #[arg(long)]
    tx_index: Option<usize>,
    #[arg(long, value_enum, default_value_t = TxPosition::Before)]
    tx_position: TxPosition,
    #[arg(long)]
    overlay_vault_code_hex: Option<String>,
    #[arg(long)]
    synthetic_vault_token_balance_raw: Option<String>,
    #[arg(long)]
    synthetic_owner_eth_wei: Option<String>,
    #[arg(long, default_value = DEFAULT_MAX_FEE_PER_GAS_WEI)]
    max_fee_per_gas_wei: String,
    #[arg(long, default_value = DEFAULT_PRIORITY_FEE_PER_GAS_WEI)]
    priority_fee_per_gas_wei: String,
    #[arg(long, default_value_t = false)]
    trace_failure: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let owner = parse_address(&args.owner, "owner")?;
    let vault = parse_address(&args.vault, "vault")?;
    let token = parse_address(&args.token, "token")?;
    let transfer_amount = parse_u256(&args.transfer_amount_raw, "transfer_amount_raw")?;
    let requested_sell_amount = args
        .sell_amount_raw
        .as_deref()
        .map(|value| parse_u256(value, "sell_amount_raw"))
        .transpose()?;
    let synthetic_vault_token_balance = args
        .synthetic_vault_token_balance_raw
        .as_deref()
        .map(|value| parse_u256(value, "synthetic_vault_token_balance_raw"))
        .transpose()?;
    let synthetic_owner_eth = args
        .synthetic_owner_eth_wei
        .as_deref()
        .map(|value| parse_u256(value, "synthetic_owner_eth_wei"))
        .transpose()?;
    let overlay_vault_code = args
        .overlay_vault_code_hex
        .as_deref()
        .map(|value| parse_hex_bytes(value, "overlay_vault_code_hex"))
        .transpose()?;
    let buy_eth = parse_u256(&args.buy_eth_wei, "buy_eth_wei")?;
    let min_tokens_out = parse_u256(&args.min_tokens_out_raw, "min_tokens_out_raw")?;
    let min_eth_out = parse_u256(&args.min_eth_out_wei, "min_eth_out_wei")?;
    let max_fee_per_gas = parse_u128(&args.max_fee_per_gas_wei, "max_fee_per_gas_wei")?;
    let priority_fee_per_gas =
        parse_u128(&args.priority_fee_per_gas_wei, "priority_fee_per_gas_wei")?;

    let reth_datadir = match std::env::var("RETH_DATADIR") {
        Ok(path) => path,
        Err(_) => tx_simulator::config::repo::reth_datadir()?,
    };
    let simulator = TxSimulator::new(&reth_datadir)?;
    let latest_block = simulator.latest_historical_context_block_number()?;
    let block = args.block.unwrap_or(latest_block);
    if block > latest_block {
        return Err(eyre!(
            "requested block {block} is ahead of latest local simulation context {latest_block}"
        ));
    }

    let (mut chain, coordinate_tx_hash) = if let Some(tx_index) = args.tx_index {
        let mut session = simulator.block_tx_state_session(block).await?;
        let tx_hash = session.transaction_hash(tx_index)?;
        let chain = match args.tx_position {
            TxPosition::Before => session.simulation_chain_before_tx(tx_index)?,
            TxPosition::After => session.simulation_chain_after_tx(tx_index)?,
        };
        (chain, Some(tx_hash.to_string()))
    } else {
        (simulator.start_simulation_chain(Some(block)).await?, None)
    };
    let base_fee_per_gas = chain.block_base_fee();

    let vault_code_was_overlayed = if let Some(code) = overlay_vault_code {
        if code.is_empty() {
            return Err(eyre!("overlay_vault_code_hex decoded to empty bytecode"));
        }
        chain.set_account_code(vault, code)?;
        true
    } else {
        false
    };
    let owner_eth_previous = if let Some(balance) = synthetic_owner_eth {
        Some(chain.set_eth_balance(owner, balance)?)
    } else {
        None
    };
    let synthetic_vault_balance_slot = if let Some(balance) = synthetic_vault_token_balance {
        Some(
            inject_standard_erc20_balance(&mut chain, token, vault, balance)?.ok_or_else(|| {
                eyre!("could not inject synthetic vault token balance for token {token}")
            })?,
        )
    } else {
        None
    };

    let balances_before = balances(&mut chain, token, owner, vault)?;

    let mut buy_result = None;
    let mut transfer_result = None;
    let mut sell_result = None;

    let vault_tokens_available = match args.scenario {
        Scenario::WalletTransferThenSell => {
            if balances_before.owner_token < transfer_amount {
                return Err(eyre!(
                    "owner token balance {} is below requested transfer amount {}",
                    balances_before.owner_token,
                    transfer_amount
                ));
            }
            let mut transfer_tx = erc20_transfer_tx(owner, token, vault, transfer_amount);
            apply_eip1559_gas_policy(&mut transfer_tx, max_fee_per_gas, priority_fee_per_gas);
            let result = chain
                .step_with_trace(transfer_tx)
                .await
                .wrap_err("wallet token transfer simulation failed")?;
            maybe_print_trace("wallet transfer", &result, args.trace_failure);
            transfer_result = Some(result);
            let vault_token_after_transfer = erc20_balance(&mut chain, token, vault)?;
            vault_token_after_transfer.saturating_sub(balances_before.vault_token)
        }
        Scenario::BuyThenSell => {
            let mut buy_tx = build_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens(
                vault,
                owner,
                token,
                buy_eth,
                min_tokens_out,
                args.deadline,
            );
            apply_eip1559_gas_policy(&mut buy_tx, max_fee_per_gas, priority_fee_per_gas);
            let result = chain
                .step_with_trace(buy_tx)
                .await
                .wrap_err("vault buy simulation failed")?;
            maybe_print_trace("vault buy", &result, args.trace_failure);
            buy_result = Some(result);
            let vault_token_after_buy = erc20_balance(&mut chain, token, vault)?;
            vault_token_after_buy.saturating_sub(balances_before.vault_token)
        }
        Scenario::CurrentStateSell => balances_before.vault_token,
    };

    let sell_amount = requested_sell_amount.unwrap_or_else(|| match args.scenario {
        Scenario::CurrentStateSell => vault_tokens_available,
        Scenario::WalletTransferThenSell | Scenario::BuyThenSell => vault_tokens_available,
    });

    let can_attempt_sell = match args.scenario {
        Scenario::WalletTransferThenSell => transfer_result
            .as_ref()
            .map(|result| result.success)
            .unwrap_or(false),
        Scenario::BuyThenSell => buy_result
            .as_ref()
            .map(|result| result.success)
            .unwrap_or(false),
        Scenario::CurrentStateSell => true,
    } && !sell_amount.is_zero();

    if can_attempt_sell {
        let mut sell_tx = build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
            vault,
            owner,
            token,
            sell_amount,
            min_eth_out,
            args.deadline,
        );
        apply_eip1559_gas_policy(&mut sell_tx, max_fee_per_gas, priority_fee_per_gas);
        let result = chain
            .step_with_trace(sell_tx)
            .await
            .wrap_err("vault emergency sell simulation failed")?;
        maybe_print_trace("vault sell", &result, args.trace_failure);
        sell_result = Some(result);
    }

    let balances_after = balances(&mut chain, token, owner, vault)?;
    let state = chain.current_state();
    let report = json!({
        "schema": "uniswap_v2_trading_vault_deployed_simulation_v1",
        "scenario": format!("{:?}", args.scenario),
        "reth_datadir": reth_datadir,
        "latest_context_block": latest_block,
        "block": block,
        "state_coordinate": {
            "tx_index": args.tx_index,
            "tx_position": args.tx_index.map(|_| format!("{:?}", args.tx_position)),
            "tx_hash": coordinate_tx_hash,
        },
        "owner": owner.to_string(),
        "vault": vault.to_string(),
        "token": token.to_string(),
        "symbol": args.symbol,
        "deadline": args.deadline,
        "inputs": {
            "transfer_amount_raw": transfer_amount.to_string(),
            "buy_eth_wei": buy_eth.to_string(),
            "min_tokens_out_raw": min_tokens_out.to_string(),
            "min_eth_out_wei": min_eth_out.to_string(),
            "sell_amount_raw": sell_amount.to_string()
        },
        "overrides": {
            "vault_code_overlayed": vault_code_was_overlayed,
            "synthetic_vault_token_balance_raw": synthetic_vault_token_balance.map(|value| value.to_string()),
            "synthetic_vault_balance_slot": synthetic_vault_balance_slot,
            "synthetic_owner_eth_wei": synthetic_owner_eth.map(|value| value.to_string()),
            "synthetic_owner_eth_previous_wei": owner_eth_previous.map(|value| value.to_string())
        },
        "gas_policy": {
            "kind": "eip1559_public_priority_fee",
            "max_fee_per_gas_wei": max_fee_per_gas.to_string(),
            "priority_fee_per_gas_wei": priority_fee_per_gas.to_string(),
            "block_base_fee_per_gas_wei": base_fee_per_gas.map(|value| value.to_string()),
            "direct_coinbase_payment_supported_by_deployed_vault": false,
            "note": "This models the current v1 bribe path: public EIP-1559 priority fee. Direct block.coinbase or private bundle bribes require a new contract/protocol path."
        },
        "balances_before": balances_before.to_json(),
        "buy": buy_result.as_ref().map(|result| tx_report(result, base_fee_per_gas)),
        "transfer": transfer_result.as_ref().map(|result| tx_report(result, base_fee_per_gas)),
        "vault_sell": sell_result.as_ref().map(|result| tx_report(result, base_fee_per_gas)),
        "balances_after": balances_after.to_json(),
        "chain_state": {
            "transactions_executed": state.transaction_count,
            "total_gas_used": state.total_gas_used
        }
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[derive(Debug, Clone)]
struct BalanceSnapshot {
    owner_token: U256,
    vault_token: U256,
    owner_eth: U256,
    vault_eth: U256,
}

impl BalanceSnapshot {
    fn to_json(&self) -> Value {
        json!({
            "owner_token_raw": self.owner_token.to_string(),
            "vault_token_raw": self.vault_token.to_string(),
            "owner_eth_wei": self.owner_eth.to_string(),
            "vault_eth_wei": self.vault_eth.to_string()
        })
    }
}

fn balances(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
    vault: Address,
) -> Result<BalanceSnapshot> {
    Ok(BalanceSnapshot {
        owner_token: erc20_balance(chain, token, owner)?,
        vault_token: erc20_balance(chain, token, vault)?,
        owner_eth: chain.eth_balance(owner)?,
        vault_eth: chain.eth_balance(vault)?,
    })
}

fn tx_report(result: &FullSimulationResult, base_fee_per_gas: Option<u128>) -> Value {
    let effective_gas_price = result.effective_gas_price.unwrap_or_default();
    let effective_priority_fee = base_fee_per_gas
        .map(|base_fee| effective_gas_price.saturating_sub(base_fee))
        .unwrap_or_default();
    let priority_spend = U256::from(result.gas_used) * U256::from(effective_priority_fee);
    json!({
        "success": result.success,
        "gas_used": result.gas_used,
        "logs": result.logs.len(),
        "revert_reason": result.revert_reason,
        "effective_gas_price_wei": result.effective_gas_price.map(|value| value.to_string()),
        "effective_priority_fee_per_gas_wei": effective_priority_fee.to_string(),
        "estimated_priority_fee_spend_wei": priority_spend.to_string()
    })
}

fn parse_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value).wrap_err_with(|| format!("invalid {label} address {value:?}"))
}

fn parse_u256(value: &str, label: &str) -> Result<U256> {
    U256::from_str(value).wrap_err_with(|| format!("invalid {label} value {value:?}"))
}

fn parse_u128(value: &str, label: &str) -> Result<u128> {
    value
        .parse::<u128>()
        .wrap_err_with(|| format!("invalid {label} value {value:?}"))
}

fn parse_hex_bytes(value: &str, label: &str) -> Result<Bytes> {
    let trimmed = value.trim();
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let decoded = hex::decode(without_prefix)
        .wrap_err_with(|| format!("invalid {label} hex value length={}", trimmed.len()))?;
    Ok(Bytes::from(decoded))
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

fn inject_standard_erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token: Address,
    owner: Address,
    amount: U256,
) -> Result<Option<u64>> {
    for slot in 0..64 {
        let storage_key = standard_erc20_balance_storage_key(owner, slot);
        let previous = chain.set_account_storage(token, storage_key, amount)?;
        let observed = erc20_balance(chain, token, owner)?;
        if observed == amount {
            return Ok(Some(slot));
        }
        chain.set_account_storage(token, storage_key, previous)?;
    }
    Ok(None)
}

fn standard_erc20_balance_storage_key(owner: Address, slot: u64) -> U256 {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(owner.into_word().as_slice());
    input[32..].copy_from_slice(&U256::from(slot).to_be_bytes::<32>());
    U256::from_be_slice(keccak256(input).as_slice())
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
        value: Some(U256::ZERO),
        data: Some(Bytes::from(
            transferCall {
                to: recipient,
                amount,
            }
            .abi_encode(),
        )),
        ..Default::default()
    }
}

fn apply_eip1559_gas_policy(
    tx: &mut UnsignedTransaction,
    max_fee_per_gas: u128,
    priority_fee_per_gas: u128,
) {
    tx.gas_price = None;
    tx.max_fee_per_gas = Some(max_fee_per_gas);
    tx.max_priority_fee_per_gas = Some(priority_fee_per_gas);
}

fn maybe_print_trace(label: &str, result: &FullSimulationResult, enabled: bool) {
    if enabled && !result.success {
        eprintln!("{label} call trace:\n{:#?}", result.call_trace);
    }
}

fn decode_u256(output: &Bytes, label: &str) -> Result<U256> {
    if output.len() < 32 {
        return Err(eyre!("{label} is shorter than 32 bytes"));
    }
    Ok(U256::from_be_slice(&output[..32]))
}
