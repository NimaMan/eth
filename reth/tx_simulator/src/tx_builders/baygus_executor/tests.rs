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

    let input = encode_permit2_signature_transfer_from_input(BaygusPermit2SignatureTransferFrom {
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
