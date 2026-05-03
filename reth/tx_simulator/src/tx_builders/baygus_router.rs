use crate::UnsignedTransaction;
use alloy_primitives::{Address, Bytes, B256, U256};

const WORD_BYTES: usize = 32;

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

/// BaygusRouter command bytes from `soleth/baygus-router/contracts/src/types/SharedTypes.sol`.
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

/// A typed command plan for `BaygusRouter.execute(bytes,bytes[])`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaygusExecutePlan {
    commands: Vec<BaygusCommand>,
    inputs: Vec<Bytes>,
    eth_value: U256,
}

impl BaygusExecutePlan {
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

    pub fn transfer_from_owner(
        &mut self,
        token: Address,
        owner: Address,
        amount: U256,
    ) -> &mut Self {
        self.push_raw(
            BaygusCommand::TransferFrom,
            encode_transfer_from_owner_input(token, owner, amount),
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
        nested_plan: &BaygusExecutePlan,
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
}

pub fn build_baygus_execute_tx(
    router: Address,
    caller: Address,
    plan: &BaygusExecutePlan,
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

pub fn encode_transfer_from_owner_input(token: Address, owner: Address, amount: U256) -> Bytes {
    let mut data = Vec::with_capacity(WORD_BYTES * 3);
    data.extend_from_slice(&pad_address(token));
    data.extend_from_slice(&pad_address(owner));
    data.extend_from_slice(&pad_u256(amount));
    Bytes::from(data)
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

        let mut plan = BaygusExecutePlan::new();
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
}
