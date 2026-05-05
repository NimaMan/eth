use crate::UnsignedTransaction;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};

const WORD_BYTES: usize = 32;
const PERMIT2_APPROVE_SELECTOR: [u8; 4] = [0x87, 0x51, 0x7c, 0x45];
const BAYGUS_EXECUTION_TYPE: &str =
    "BaygusExecution(address executor,address caller,bytes32 commandsHash,bytes32 inputsHash)";
const PERMIT2_SIGNATURE_TRANSFER_INPUT_TYPE: &str = "Permit2SignatureTransferInput(address owner,address token,uint256 permittedAmount,uint256 nonce,uint256 deadline,uint256 requestedAmount)";

pub const BAYGUS_EXECUTION_WITNESS_TYPE: &str =
    "BaygusExecution witness)BaygusExecution(address executor,address caller,bytes32 commandsHash,bytes32 inputsHash)TokenPermissions(address token,uint256 amount)";

pub const CMD_V4_SWAP: u8 = 0x01;
pub const CMD_V2_SWAP: u8 = 0x02;
pub const CMD_V3_SWAP: u8 = 0x03;
pub const CMD_SUSHISWAP: u8 = 0x04;
pub const CMD_CURVE_SWAP: u8 = 0x05;
pub const CMD_BALANCER_SWAP: u8 = 0x06;
pub const CMD_SWEEP: u8 = 0x07;
pub const CMD_BALANCER_FLASH_LOAN: u8 = 0x08;
pub const CMD_PERMIT2_TRANSFER_FROM: u8 = 0x09;
pub const CMD_TRANSFER_FROM: u8 = 0x0a;
pub const CMD_COINBASE_TIP: u8 = 0x0b;
pub const CMD_PERMIT2_SIGNATURE_TRANSFER_FROM: u8 = 0x0c;
pub const CMD_V2_PAIR_SWAP: u8 = 0x0d;

/// BaygusExecutor command bytes from `soleth/baygus-executor/contracts/src/types/SharedTypes.sol`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BaygusCommand {
    V4Swap = CMD_V4_SWAP,
    V2Swap = CMD_V2_SWAP,
    V3Swap = CMD_V3_SWAP,
    Sushiswap = CMD_SUSHISWAP,
    CurveSwap = CMD_CURVE_SWAP,
    BalancerSwap = CMD_BALANCER_SWAP,
    Sweep = CMD_SWEEP,
    BalancerFlashLoan = CMD_BALANCER_FLASH_LOAN,
    Permit2TransferFrom = CMD_PERMIT2_TRANSFER_FROM,
    TransferFrom = CMD_TRANSFER_FROM,
    CoinbaseTip = CMD_COINBASE_TIP,
    Permit2SignatureTransferFrom = CMD_PERMIT2_SIGNATURE_TRANSFER_FROM,
    V2PairSwap = CMD_V2_PAIR_SWAP,
}

impl BaygusCommand {
    pub const fn byte(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaygusV3ExactInputSingle {
    pub token_in: Address,
    pub token_out: Address,
    pub fee: u32,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_in: U256,
    pub amount_out_minimum: U256,
    pub sqrt_price_limit_x96: U256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaygusCurveSwap {
    pub pool: Address,
    pub token_in: Address,
    pub token_out: Address,
    pub recipient: Address,
    pub i: i128,
    pub j: i128,
    pub dx: U256,
    pub min_dy: U256,
    pub use_underlying: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaygusBalancerSwap {
    pub pool_id: B256,
    pub asset_in: Address,
    pub asset_out: Address,
    pub recipient: Address,
    pub amount: U256,
    pub limit: U256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaygusV2PairSwap {
    pub pair: Address,
    pub token_in: Address,
    pub amount_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
    pub recipient: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaygusPermit2SignatureTransferFrom {
    /// Use `Address::ZERO` to default to the executor caller. Non-zero owners are safe here
    /// because Permit2 verifies a signature over the exact Baygus plan witness.
    pub owner: Address,
    pub token: Address,
    pub permitted_amount: U256,
    pub nonce: U256,
    pub deadline: U256,
    pub requested_amount: U256,
    pub signature: Bytes,
}

/// A typed command plan for `BaygusExecutor.execute(bytes,bytes[])`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaygusExecutionPlan {
    commands: Vec<BaygusCommand>,
    inputs: Vec<Bytes>,
    eth_value: U256,
}

#[deprecated(note = "use BaygusExecutionPlan")]
pub type BaygusExecutePlan = BaygusExecutionPlan;

impl BaygusExecutionPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_eth_value(mut self, value: U256) -> Self {
        self.eth_value = value;
        self
    }

    pub fn set_eth_value(&mut self, value: U256) -> &mut Self {
        self.eth_value = value;
        self
    }

    pub fn eth_value(&self) -> U256 {
        self.eth_value
    }

    pub fn push_raw(&mut self, command: BaygusCommand, input: impl Into<Bytes>) -> &mut Self {
        self.commands.push(command);
        self.inputs.push(input.into());
        self
    }

    pub fn transfer_from(&mut self, token: Address, amount: U256) -> &mut Self {
        self.push_raw(
            BaygusCommand::TransferFrom,
            encode_transfer_from_input(token, amount),
        )
    }

    pub fn permit2_transfer_from(&mut self, token: Address, amount: U256) -> &mut Self {
        self.push_raw(
            BaygusCommand::Permit2TransferFrom,
            encode_permit2_transfer_from_input(token, amount),
        )
    }

    pub fn permit2_signature_transfer_from(
        &mut self,
        params: BaygusPermit2SignatureTransferFrom,
    ) -> &mut Self {
        self.push_raw(
            BaygusCommand::Permit2SignatureTransferFrom,
            encode_permit2_signature_transfer_from_input(params),
        )
    }

    pub fn v2_swap(
        &mut self,
        amount_in: U256,
        amount_out_min: U256,
        path: impl Into<Vec<Address>>,
        recipient: Address,
    ) -> &mut Self {
        self.push_raw(
            BaygusCommand::V2Swap,
            encode_v2_swap_input(amount_in, amount_out_min, path, recipient),
        )
    }

    pub fn sushiswap_swap(
        &mut self,
        amount_in: U256,
        amount_out_min: U256,
        path: impl Into<Vec<Address>>,
        recipient: Address,
    ) -> &mut Self {
        self.push_raw(
            BaygusCommand::Sushiswap,
            encode_v2_swap_input(amount_in, amount_out_min, path, recipient),
        )
    }

    pub fn v2_pair_swap(&mut self, params: BaygusV2PairSwap) -> &mut Self {
        self.push_raw(BaygusCommand::V2PairSwap, encode_v2_pair_swap_input(params))
    }

    pub fn v3_swap(&mut self, params: BaygusV3ExactInputSingle) -> &mut Self {
        self.push_raw(BaygusCommand::V3Swap, encode_v3_swap_input(params))
    }

    pub fn curve_swap(&mut self, params: BaygusCurveSwap) -> &mut Self {
        self.push_raw(BaygusCommand::CurveSwap, encode_curve_swap_input(params))
    }

    pub fn balancer_swap(&mut self, params: BaygusBalancerSwap) -> &mut Self {
        self.push_raw(
            BaygusCommand::BalancerSwap,
            encode_balancer_swap_input(params),
        )
    }

    pub fn balancer_flash_loan(
        &mut self,
        tokens: impl Into<Vec<Address>>,
        amounts: impl Into<Vec<U256>>,
        nested_plan: &BaygusExecutionPlan,
    ) -> &mut Self {
        let user_data = nested_plan.execute_args();
        self.push_raw(
            BaygusCommand::BalancerFlashLoan,
            encode_balancer_flash_loan_input(tokens, amounts, user_data),
        )
    }

    pub fn v4_unlock(&mut self, unlock_data: impl Into<Bytes>) -> &mut Self {
        self.push_raw(BaygusCommand::V4Swap, unlock_data)
    }

    pub fn sweep(&mut self, token: Address, recipient: Address, minimum_amount: U256) -> &mut Self {
        self.push_raw(
            BaygusCommand::Sweep,
            encode_sweep_input(token, recipient, minimum_amount),
        )
    }

    pub fn coinbase_tip(&mut self, amount: U256) -> &mut Self {
        self.eth_value += amount;
        self.push_raw(
            BaygusCommand::CoinbaseTip,
            encode_coinbase_tip_input(amount),
        )
    }

    pub fn coinbase_tip_with_block_guard(
        &mut self,
        amount: U256,
        min_block: U256,
        max_block: U256,
    ) -> &mut Self {
        self.eth_value += amount;
        self.push_raw(
            BaygusCommand::CoinbaseTip,
            encode_coinbase_tip_with_block_guard_input(amount, min_block, max_block),
        )
    }

    pub fn commands(&self) -> &[BaygusCommand] {
        &self.commands
    }

    pub fn commands_bytes(&self) -> Bytes {
        Bytes::from(
            self.commands
                .iter()
                .map(|command| command.byte())
                .collect::<Vec<_>>(),
        )
    }

    pub fn inputs(&self) -> &[Bytes] {
        &self.inputs
    }

    pub fn execute_args(&self) -> Bytes {
        let commands = self.commands_bytes();
        encode_execute_args(commands.as_ref(), &self.inputs)
    }

    pub fn calldata(&self) -> Bytes {
        let commands = self.commands_bytes();
        encode_execute(commands.as_ref(), &self.inputs)
    }

    pub fn to_transaction(&self, router: Address, caller: Address) -> UnsignedTransaction {
        build_baygus_execute_tx(router, caller, self)
    }

    pub fn plan_witness(&self, executor: Address, caller: Address) -> B256 {
        baygus_plan_witness(executor, caller, self)
    }
}

pub fn build_baygus_execute_tx(
    router: Address,
    caller: Address,
    plan: &BaygusExecutionPlan,
) -> UnsignedTransaction {
    UnsignedTransaction {
        from: Some(caller),
        to: Some(router),
        gas: Some(5_000_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(plan.eth_value()),
        data: Some(plan.calldata()),
        nonce: None,
        ..Default::default()
    }
}

pub fn build_permit2_approve_tx(
    owner: Address,
    permit2: Address,
    token: Address,
    spender: Address,
    amount: U256,
    expiration: u64,
) -> UnsignedTransaction {
    assert!(
        amount <= max_uint160(),
        "Permit2 allowance amount must fit uint160"
    );
    assert!(
        expiration <= max_uint48(),
        "Permit2 expiration must fit uint48"
    );

    let mut data = Vec::with_capacity(4 + WORD_BYTES * 4);
    data.extend_from_slice(&PERMIT2_APPROVE_SELECTOR);
    data.extend_from_slice(&pad_address(token));
    data.extend_from_slice(&pad_address(spender));
    data.extend_from_slice(&pad_u256(amount));
    data.extend_from_slice(&pad_u256(U256::from(expiration)));

    UnsignedTransaction {
        from: Some(owner),
        to: Some(permit2),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    }
}

pub fn encode_execute(commands: &[u8], inputs: &[Bytes]) -> Bytes {
    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&[0x24, 0x85, 0x6b, 0xc3]);
    data.extend_from_slice(encode_execute_args(commands, inputs).as_ref());
    Bytes::from(data)
}

pub fn encode_execute_args(commands: &[u8], inputs: &[Bytes]) -> Bytes {
    let commands_tail = encode_dynamic_bytes(commands);
    let inputs_tail = encode_bytes_array(inputs);
    let inputs_offset = WORD_BYTES * 2 + commands_tail.len();

    let mut data = Vec::with_capacity(WORD_BYTES * 2 + commands_tail.len() + inputs_tail.len());
    data.extend_from_slice(&pad_usize(WORD_BYTES * 2));
    data.extend_from_slice(&pad_usize(inputs_offset));
    data.extend_from_slice(&commands_tail);
    data.extend_from_slice(&inputs_tail);
    Bytes::from(data)
}

pub fn encode_transfer_from_input(token: Address, amount: U256) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 2);
    data.extend_from_slice(&pad_address(token));
    data.extend_from_slice(&pad_u256(amount));
    Bytes::from(data)
}

pub fn encode_permit2_transfer_from_input(token: Address, amount: U256) -> Bytes {
    encode_transfer_from_input(token, amount)
}

pub fn encode_permit2_signature_transfer_from_input(
    params: BaygusPermit2SignatureTransferFrom,
) -> Bytes {
    let encoded_signature = encode_dynamic_bytes(params.signature.as_ref());
    let mut data = Vec::with_capacity(WORD_BYTES * 7 + encoded_signature.len());
    data.extend_from_slice(&pad_address(params.owner));
    data.extend_from_slice(&pad_address(params.token));
    data.extend_from_slice(&pad_u256(params.permitted_amount));
    data.extend_from_slice(&pad_u256(params.nonce));
    data.extend_from_slice(&pad_u256(params.deadline));
    data.extend_from_slice(&pad_u256(params.requested_amount));
    data.extend_from_slice(&pad_usize(WORD_BYTES * 7));
    data.extend_from_slice(&encoded_signature);
    Bytes::from(data)
}

pub fn baygus_plan_witness(executor: Address, caller: Address, plan: &BaygusExecutionPlan) -> B256 {
    let commands = plan.commands_bytes();
    let inputs_hash = baygus_inputs_hash(commands.as_ref(), plan.inputs(), caller);
    let mut data = Vec::with_capacity(WORD_BYTES * 5);
    data.extend_from_slice(keccak256(BAYGUS_EXECUTION_TYPE.as_bytes()).as_slice());
    data.extend_from_slice(&pad_address(executor));
    data.extend_from_slice(&pad_address(caller));
    data.extend_from_slice(keccak256(commands.as_ref()).as_slice());
    data.extend_from_slice(inputs_hash.as_slice());
    keccak256(data)
}

pub fn baygus_inputs_hash(commands: &[u8], inputs: &[Bytes], caller: Address) -> B256 {
    assert_eq!(
        commands.len(),
        inputs.len(),
        "Baygus commands and inputs must have the same length"
    );

    let mut data = Vec::with_capacity(WORD_BYTES * inputs.len());
    for (command, input) in commands.iter().zip(inputs) {
        let input_hash = if *command == CMD_PERMIT2_SIGNATURE_TRANSFER_FROM {
            permit2_signature_transfer_input_hash(input, caller)
        } else {
            keccak256(input.as_ref())
        };
        data.extend_from_slice(input_hash.as_slice());
    }
    keccak256(data)
}

pub fn permit2_signature_transfer_input_hash(input: &Bytes, caller: Address) -> B256 {
    assert!(
        input.len() >= WORD_BYTES * 8,
        "Permit2 signature transfer input must contain ABI head and signature tail"
    );

    let mut owner = decode_address_word(input.as_ref(), 0);
    if owner == Address::ZERO {
        owner = caller;
    }

    let mut data = Vec::with_capacity(WORD_BYTES * 7);
    data.extend_from_slice(keccak256(PERMIT2_SIGNATURE_TRANSFER_INPUT_TYPE.as_bytes()).as_slice());
    data.extend_from_slice(&pad_address(owner));
    data.extend_from_slice(word(input.as_ref(), 1));
    data.extend_from_slice(word(input.as_ref(), 2));
    data.extend_from_slice(word(input.as_ref(), 3));
    data.extend_from_slice(word(input.as_ref(), 4));
    data.extend_from_slice(word(input.as_ref(), 5));
    keccak256(data)
}

pub fn encode_v2_swap_input(
    amount_in: U256,
    amount_out_min: U256,
    path: impl Into<Vec<Address>>,
    recipient: Address,
) -> Bytes {
    let path = path.into();
    assert!(
        path.len() >= 2,
        "Baygus V2/Sushiswap path must contain at least input and output tokens"
    );

    let encoded_path = encode_address_array(&path);
    let mut data = Vec::with_capacity(WORD_BYTES * 4 + encoded_path.len());
    data.extend_from_slice(&pad_u256(amount_in));
    data.extend_from_slice(&pad_u256(amount_out_min));
    data.extend_from_slice(&pad_usize(WORD_BYTES * 4));
    data.extend_from_slice(&pad_address(recipient));
    data.extend_from_slice(&encoded_path);
    Bytes::from(data)
}

pub fn encode_v2_pair_swap_input(params: BaygusV2PairSwap) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 6);
    data.extend_from_slice(&pad_address(params.pair));
    data.extend_from_slice(&pad_address(params.token_in));
    data.extend_from_slice(&pad_u256(params.amount_in));
    data.extend_from_slice(&pad_u256(params.amount0_out));
    data.extend_from_slice(&pad_u256(params.amount1_out));
    data.extend_from_slice(&pad_address(params.recipient));
    Bytes::from(data)
}

pub fn encode_v3_swap_input(params: BaygusV3ExactInputSingle) -> Bytes {
    assert!(params.fee <= 0x00ff_ffff, "Uniswap V3 fee must fit uint24");

    let mut data = Vec::with_capacity(WORD_BYTES * 8);
    data.extend_from_slice(&pad_address(params.token_in));
    data.extend_from_slice(&pad_address(params.token_out));
    data.extend_from_slice(&pad_u256(U256::from(params.fee)));
    data.extend_from_slice(&pad_address(params.recipient));
    data.extend_from_slice(&pad_u256(params.deadline));
    data.extend_from_slice(&pad_u256(params.amount_in));
    data.extend_from_slice(&pad_u256(params.amount_out_minimum));
    data.extend_from_slice(&pad_u256(params.sqrt_price_limit_x96));
    Bytes::from(data)
}

pub fn encode_curve_swap_input(params: BaygusCurveSwap) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 9);
    data.extend_from_slice(&pad_address(params.pool));
    data.extend_from_slice(&pad_address(params.token_in));
    data.extend_from_slice(&pad_address(params.token_out));
    data.extend_from_slice(&pad_address(params.recipient));
    data.extend_from_slice(&pad_i128(params.i));
    data.extend_from_slice(&pad_i128(params.j));
    data.extend_from_slice(&pad_u256(params.dx));
    data.extend_from_slice(&pad_u256(params.min_dy));
    data.extend_from_slice(&pad_bool(params.use_underlying));
    Bytes::from(data)
}

pub fn encode_balancer_swap_input(params: BaygusBalancerSwap) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 6);
    data.extend_from_slice(params.pool_id.as_slice());
    data.extend_from_slice(&pad_address(params.asset_in));
    data.extend_from_slice(&pad_address(params.asset_out));
    data.extend_from_slice(&pad_address(params.recipient));
    data.extend_from_slice(&pad_u256(params.amount));
    data.extend_from_slice(&pad_u256(params.limit));
    Bytes::from(data)
}

pub fn encode_sweep_input(token: Address, recipient: Address, minimum_amount: U256) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 3);
    data.extend_from_slice(&pad_address(token));
    data.extend_from_slice(&pad_address(recipient));
    data.extend_from_slice(&pad_u256(minimum_amount));
    Bytes::from(data)
}

pub fn encode_coinbase_tip_input(amount: U256) -> Bytes {
    Bytes::from(pad_u256(amount).to_vec())
}

pub fn encode_coinbase_tip_with_block_guard_input(
    amount: U256,
    min_block: U256,
    max_block: U256,
) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 3);
    data.extend_from_slice(&pad_u256(amount));
    data.extend_from_slice(&pad_u256(min_block));
    data.extend_from_slice(&pad_u256(max_block));
    Bytes::from(data)
}

pub fn encode_balancer_flash_loan_input(
    tokens: impl Into<Vec<Address>>,
    amounts: impl Into<Vec<U256>>,
    user_data: impl Into<Bytes>,
) -> Bytes {
    let tokens = tokens.into();
    let amounts = amounts.into();
    assert_eq!(
        tokens.len(),
        amounts.len(),
        "Balancer flash loan tokens and amounts must have the same length"
    );

    let encoded_tokens = encode_address_array(&tokens);
    let encoded_amounts = encode_u256_array(&amounts);
    let user_data = user_data.into();
    let encoded_user_data = encode_dynamic_bytes(user_data.as_ref());
    let amounts_offset = WORD_BYTES * 3 + encoded_tokens.len();
    let user_data_offset = amounts_offset + encoded_amounts.len();

    let mut data = Vec::with_capacity(
        WORD_BYTES * 3 + encoded_tokens.len() + encoded_amounts.len() + encoded_user_data.len(),
    );
    data.extend_from_slice(&pad_usize(WORD_BYTES * 3));
    data.extend_from_slice(&pad_usize(amounts_offset));
    data.extend_from_slice(&pad_usize(user_data_offset));
    data.extend_from_slice(&encoded_tokens);
    data.extend_from_slice(&encoded_amounts);
    data.extend_from_slice(&encoded_user_data);
    Bytes::from(data)
}

fn encode_address_array(addresses: &[Address]) -> Vec<u8> {
    let mut data = Vec::with_capacity(WORD_BYTES + WORD_BYTES * addresses.len());
    data.extend_from_slice(&pad_usize(addresses.len()));
    for address in addresses {
        data.extend_from_slice(&pad_address(*address));
    }
    data
}

fn encode_u256_array(values: &[U256]) -> Vec<u8> {
    let mut data = Vec::with_capacity(WORD_BYTES + WORD_BYTES * values.len());
    data.extend_from_slice(&pad_usize(values.len()));
    for value in values {
        data.extend_from_slice(&pad_u256(*value));
    }
    data
}

fn encode_bytes_array(inputs: &[Bytes]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&pad_usize(inputs.len()));

    let mut tails = Vec::with_capacity(inputs.len());
    let mut tail_offset = WORD_BYTES * inputs.len();
    for input in inputs {
        data.extend_from_slice(&pad_usize(tail_offset));
        let tail = encode_dynamic_bytes(input.as_ref());
        tail_offset += tail.len();
        tails.push(tail);
    }

    for tail in tails {
        data.extend_from_slice(&tail);
    }

    data
}

fn encode_dynamic_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(WORD_BYTES + padded_len(bytes.len()));
    data.extend_from_slice(&pad_usize(bytes.len()));
    data.extend_from_slice(bytes);
    data.resize(WORD_BYTES + padded_len(bytes.len()), 0);
    data
}

fn padded_len(len: usize) -> usize {
    ((len + WORD_BYTES - 1) / WORD_BYTES) * WORD_BYTES
}

fn pad_usize(value: usize) -> [u8; WORD_BYTES] {
    U256::from(value as u64).to_be_bytes()
}

fn pad_address(address: Address) -> [u8; WORD_BYTES] {
    let mut out = [0u8; WORD_BYTES];
    out[12..].copy_from_slice(address.as_slice());
    out
}

fn pad_u256(value: U256) -> [u8; WORD_BYTES] {
    value.to_be_bytes()
}

fn pad_i128(value: i128) -> [u8; WORD_BYTES] {
    let mut out = if value < 0 {
        [0xffu8; WORD_BYTES]
    } else {
        [0u8; WORD_BYTES]
    };
    out[16..].copy_from_slice(&value.to_be_bytes());
    out
}

fn pad_bool(value: bool) -> [u8; WORD_BYTES] {
    let mut out = [0u8; WORD_BYTES];
    out[31] = u8::from(value);
    out
}

fn word(data: &[u8], index: usize) -> &[u8] {
    let start = index * WORD_BYTES;
    let end = start + WORD_BYTES;
    data.get(start..end)
        .expect("ABI word index must be in bounds")
}

fn decode_address_word(data: &[u8], index: usize) -> Address {
    let mut address = [0u8; 20];
    address.copy_from_slice(&word(data, index)[12..]);
    Address::from(address)
}

fn max_uint160() -> U256 {
    (U256::from(1u8) << 160) - U256::from(1u8)
}

fn max_uint48() -> u64 {
    (1u64 << 48) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(byte: u8) -> Address {
        Address::from([byte; 20])
    }

    #[test]
    fn execute_calldata_uses_selector_and_offsets() {
        let input = encode_transfer_from_input(address(0x11), U256::from(100));
        let calldata = encode_execute(&[CMD_TRANSFER_FROM], &[input]);

        assert_eq!(&calldata[..4], &[0x24, 0x85, 0x6b, 0xc3]);
        assert_eq!((calldata.len() - 4) % WORD_BYTES, 0);
        assert_eq!(&calldata[4 + 31..4 + 32], &[0x40]);
        assert_eq!(&calldata[4 + 63..4 + 64], &[0x80]);
    }

    #[test]
    fn plan_transfer_then_v2_encodes_commands_and_inputs() {
        let token_in = address(0x11);
        let token_out = address(0x22);
        let recipient = address(0x33);

        let mut plan = BaygusExecutionPlan::new();
        plan.transfer_from(token_in, U256::from(100)).v2_swap(
            U256::from(100),
            U256::from(95),
            vec![token_in, token_out],
            recipient,
        );

        assert_eq!(
            plan.commands_bytes().as_ref(),
            &[CMD_TRANSFER_FROM, CMD_V2_SWAP]
        );
        assert_eq!(plan.inputs().len(), 2);
        assert_eq!(plan.inputs()[0].len(), WORD_BYTES * 2);
        assert_eq!(plan.inputs()[1].len(), WORD_BYTES * 7);
        assert_eq!(plan.calldata()[4 + 64 + 31], 2);
    }

    #[test]
    fn plan_transfer_then_v2_pair_swap_encodes_static_pair_input() {
        let pair = address(0xaa);
        let token_in = address(0x11);
        let recipient = address(0x33);

        let mut plan = BaygusExecutionPlan::new();
        plan.transfer_from(token_in, U256::from(100))
            .v2_pair_swap(BaygusV2PairSwap {
                pair,
                token_in,
                amount_in: U256::from(100),
                amount0_out: U256::from(95),
                amount1_out: U256::ZERO,
                recipient,
            });

        assert_eq!(
            plan.commands_bytes().as_ref(),
            &[CMD_TRANSFER_FROM, CMD_V2_PAIR_SWAP]
        );
        assert_eq!(plan.inputs().len(), 2);
        assert_eq!(plan.inputs()[1].len(), WORD_BYTES * 6);
        assert_eq!(&plan.inputs()[1][12..WORD_BYTES], pair.as_slice());
        assert_eq!(
            &plan.inputs()[1][WORD_BYTES + 12..WORD_BYTES * 2],
            token_in.as_slice()
        );
        assert_eq!(plan.inputs()[1][WORD_BYTES * 3 - 1], 100);
        assert_eq!(plan.inputs()[1][WORD_BYTES * 4 - 1], 95);
        assert_eq!(
            &plan.inputs()[1][WORD_BYTES * 5 + 12..WORD_BYTES * 6],
            recipient.as_slice()
        );
    }

    #[test]
    fn permit2_transfer_from_uses_reserved_command_byte() {
        let token = address(0x11);

        let mut plan = BaygusExecutionPlan::new();
        plan.permit2_transfer_from(token, U256::from(100));

        assert_eq!(plan.commands_bytes().as_ref(), &[CMD_PERMIT2_TRANSFER_FROM]);
        assert_eq!(plan.inputs()[0].len(), WORD_BYTES * 2);
        assert_eq!(plan.inputs()[0][WORD_BYTES * 2 - 1], 100);
    }

    #[test]
    fn permit2_signature_transfer_from_encodes_single_use_pull() {
        let owner = address(0x11);
        let token = address(0x22);
        let signature = Bytes::from(vec![0xaa, 0xbb, 0xcc]);

        let input =
            encode_permit2_signature_transfer_from_input(BaygusPermit2SignatureTransferFrom {
                owner,
                token,
                permitted_amount: U256::from(1_000),
                nonce: U256::from(7),
                deadline: U256::from(1_800_000_000u64),
                requested_amount: U256::from(400),
                signature: signature.clone(),
            });

        assert_eq!(input.len(), WORD_BYTES * 9);
        assert_eq!(&input[12..WORD_BYTES], owner.as_slice());
        assert_eq!(&input[WORD_BYTES + 12..WORD_BYTES * 2], token.as_slice());
        assert_eq!(input[WORD_BYTES * 3 - 2], 0x03);
        assert_eq!(input[WORD_BYTES * 3 - 1], 0xe8);
        assert_eq!(input[WORD_BYTES * 4 - 1], 7);
        assert_eq!(input[WORD_BYTES * 6 - 2], 0x01);
        assert_eq!(input[WORD_BYTES * 6 - 1], 0x90);
        assert_eq!(input[WORD_BYTES * 7 - 1], (WORD_BYTES * 7) as u8);
        assert_eq!(input[WORD_BYTES * 8 - 1], signature.len() as u8);
        assert_eq!(
            &input[WORD_BYTES * 8..WORD_BYTES * 8 + signature.len()],
            signature.as_ref()
        );
    }

    #[test]
    fn plan_permit2_signature_transfer_from_uses_reserved_command_byte() {
        let mut plan = BaygusExecutionPlan::new();
        plan.permit2_signature_transfer_from(BaygusPermit2SignatureTransferFrom {
            owner: Address::ZERO,
            token: address(0x11),
            permitted_amount: U256::from(100),
            nonce: U256::from(1),
            deadline: U256::from(2),
            requested_amount: U256::from(100),
            signature: Bytes::from_static(b"signature"),
        });

        assert_eq!(
            plan.commands_bytes().as_ref(),
            &[CMD_PERMIT2_SIGNATURE_TRANSFER_FROM]
        );
        assert_eq!(plan.inputs()[0][WORD_BYTES * 7 - 1], (WORD_BYTES * 7) as u8);
    }

    #[test]
    fn permit2_signature_plan_witness_ignores_signature_bytes() {
        let executor = address(0xee);
        let caller = address(0xcc);
        let token = address(0x11);

        let mut plan_a = BaygusExecutionPlan::new();
        plan_a.permit2_signature_transfer_from(BaygusPermit2SignatureTransferFrom {
            owner: Address::ZERO,
            token,
            permitted_amount: U256::from(100),
            nonce: U256::from(1),
            deadline: U256::from(2),
            requested_amount: U256::from(100),
            signature: Bytes::from_static(b"signature-a"),
        });

        let mut plan_b = BaygusExecutionPlan::new();
        plan_b.permit2_signature_transfer_from(BaygusPermit2SignatureTransferFrom {
            owner: Address::ZERO,
            token,
            permitted_amount: U256::from(100),
            nonce: U256::from(1),
            deadline: U256::from(2),
            requested_amount: U256::from(100),
            signature: Bytes::from_static(b"signature-b"),
        });

        assert_eq!(
            plan_a.plan_witness(executor, caller),
            plan_b.plan_witness(executor, caller)
        );
    }

    #[test]
    fn permit2_signature_plan_witness_binds_plan_and_caller() {
        let executor = address(0xee);
        let caller = address(0xcc);
        let token = address(0x11);
        let recipient = address(0x22);

        let mut plan = BaygusExecutionPlan::new();
        plan.permit2_signature_transfer_from(BaygusPermit2SignatureTransferFrom {
            owner: Address::ZERO,
            token,
            permitted_amount: U256::from(100),
            nonce: U256::from(1),
            deadline: U256::from(2),
            requested_amount: U256::from(100),
            signature: Bytes::from_static(b"signature"),
        })
        .sweep(token, recipient, U256::ZERO);

        let mut changed_plan = plan.clone();
        changed_plan.sweep(token, address(0x33), U256::ZERO);

        assert_ne!(
            plan.plan_witness(executor, caller),
            plan.plan_witness(executor, address(0xcd))
        );
        assert_ne!(
            plan.plan_witness(executor, caller),
            changed_plan.plan_witness(executor, caller)
        );
    }

    #[test]
    fn permit2_signature_zero_owner_hashes_as_caller() {
        let caller = address(0xcc);
        let token = address(0x11);

        let zero_owner_input =
            encode_permit2_signature_transfer_from_input(BaygusPermit2SignatureTransferFrom {
                owner: Address::ZERO,
                token,
                permitted_amount: U256::from(100),
                nonce: U256::from(1),
                deadline: U256::from(2),
                requested_amount: U256::from(100),
                signature: Bytes::from_static(b"signature"),
            });
        let explicit_owner_input =
            encode_permit2_signature_transfer_from_input(BaygusPermit2SignatureTransferFrom {
                owner: caller,
                token,
                permitted_amount: U256::from(100),
                nonce: U256::from(1),
                deadline: U256::from(2),
                requested_amount: U256::from(100),
                signature: Bytes::from_static(b"signature"),
            });

        assert_eq!(
            permit2_signature_transfer_input_hash(&zero_owner_input, caller),
            permit2_signature_transfer_input_hash(&explicit_owner_input, caller)
        );
    }

    #[test]
    fn permit2_approve_tx_encodes_allowance_transfer_approval() {
        let owner = address(0x11);
        let permit2 = address(0x22);
        let token = address(0x33);
        let spender = address(0x44);

        let tx = build_permit2_approve_tx(owner, permit2, token, spender, U256::from(100), 1234);
        let data = tx.data.expect("calldata");

        assert_eq!(tx.from, Some(owner));
        assert_eq!(tx.to, Some(permit2));
        assert_eq!(&data[..4], &PERMIT2_APPROVE_SELECTOR);
        assert_eq!(&data[4 + 12..4 + WORD_BYTES], token.as_slice());
        assert_eq!(
            &data[4 + WORD_BYTES + 12..4 + WORD_BYTES * 2],
            spender.as_slice()
        );
        assert_eq!(data[4 + WORD_BYTES * 3 - 1], 100);
        assert_eq!(data[4 + WORD_BYTES * 4 - 2], 0x04);
        assert_eq!(data[4 + WORD_BYTES * 4 - 1], 0xd2);
    }

    #[test]
    fn empty_execute_args_match_solidity_abi_shape() {
        let encoded = encode_execute_args(&[], &[]);

        assert_eq!(encoded.len(), WORD_BYTES * 4);
        assert_eq!(encoded[31], 0x40);
        assert_eq!(encoded[63], 0x60);
        assert_eq!(encoded[95], 0x00);
        assert_eq!(encoded[127], 0x00);
    }

    #[test]
    fn signed_curve_indices_are_sign_extended() {
        let encoded = encode_curve_swap_input(BaygusCurveSwap {
            pool: address(0x01),
            token_in: address(0x02),
            token_out: address(0x03),
            recipient: address(0x04),
            i: -1,
            j: 2,
            dx: U256::from(1),
            min_dy: U256::from(2),
            use_underlying: true,
        });

        let i_word = &encoded[WORD_BYTES * 4..WORD_BYTES * 5];
        let j_word = &encoded[WORD_BYTES * 5..WORD_BYTES * 6];
        assert!(i_word.iter().all(|byte| *byte == 0xff));
        assert_eq!(j_word[31], 2);
        assert_eq!(encoded[WORD_BYTES * 8 + 31], 1);
    }

    #[test]
    fn coinbase_tip_adds_command_input_and_eth_value() {
        let mut plan = BaygusExecutionPlan::new();
        plan.coinbase_tip(U256::from(10));

        assert_eq!(plan.commands_bytes().as_ref(), &[CMD_COINBASE_TIP]);
        assert_eq!(plan.eth_value(), U256::from(10));
        assert_eq!(plan.inputs()[0].len(), WORD_BYTES);
        assert_eq!(plan.inputs()[0][31], 10);
    }

    #[test]
    fn guarded_coinbase_tip_encodes_block_bounds() {
        let mut plan = BaygusExecutionPlan::new();
        plan.coinbase_tip_with_block_guard(U256::from(10), U256::from(100), U256::from(101));

        let input = &plan.inputs()[0];
        assert_eq!(plan.commands_bytes().as_ref(), &[CMD_COINBASE_TIP]);
        assert_eq!(plan.eth_value(), U256::from(10));
        assert_eq!(input.len(), WORD_BYTES * 3);
        assert_eq!(input[31], 10);
        assert_eq!(input[WORD_BYTES + 31], 100);
        assert_eq!(input[WORD_BYTES * 2 + 31], 101);
    }
}
