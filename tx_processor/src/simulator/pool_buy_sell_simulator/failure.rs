use std::convert::TryInto;
use std::sync::Arc;

use alloy_primitives::{Address, Selector, U256, hex};
use tx_simulator::types::CallFrame;
use tx_simulator::{FullSimulationResult, TxSimulator, UnsignedTransaction};

use crate::simulator::revert_decoder::describe_revert_output;

const SELECTOR_TRANSFER: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];
const SELECTOR_TRANSFER_FROM: [u8; 4] = [0x23, 0xb8, 0x72, 0xdd];
const SELECTOR_APPROVE: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
const SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS: [u8; 4] = [0x38, 0xed, 0x17, 0x39];
const SELECTOR_SWAP_EXACT_TOKENS_FOR_ETH_SUPPORTING_FEE: [u8; 4] = [0x79, 0x1a, 0xc9, 0x47];
const SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS_SUPPORTING_FEE: [u8; 4] = [0x5c, 0x11, 0xd7, 0x95];
const SELECTOR_GET_RESERVES: [u8; 4] = [0x09, 0x02, 0xf1, 0xac];

#[derive(Debug, Clone)]
struct FailureContext {
    reason: Option<String>,
    target: Option<Address>,
    selector: Option<Selector>,
    decoded_call: Option<String>,
    depth: usize,
}

pub(super) fn format_failure_with_revert(prefix: &str, revert: Option<&str>) -> String {
    let reason = revert
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(normalize_revert_text);
    match reason {
        Some(reason) => format!("{prefix} (revert: {reason})"),
        None => format!("{prefix} (empty revert payload; no decoded reason)"),
    }
}

pub(super) fn format_failure_with_full_trace(
    base_message: &str,
    full: &FullSimulationResult,
) -> String {
    let failure_context = find_failure_context(&full.call_trace, 0);
    let mut reason_opt = full
        .revert_reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty() && !is_uninformative_revert(reason))
        .map(normalize_revert_text);

    if reason_opt.is_none() {
        if let Some(ctx) = failure_context.as_ref() {
            reason_opt = ctx
                .reason
                .as_deref()
                .map(str::trim)
                .filter(|reason| !reason.is_empty() && !is_uninformative_revert(reason))
                .map(normalize_revert_text);
        }
    }

    if reason_opt.is_none() {
        reason_opt = extract_reason_from_call_trace(&full.call_trace)
            .map(|reason| normalize_revert_text(reason.trim()))
            .filter(|reason| !is_uninformative_revert(reason));
    }

    let mut message = if let Some(reason) = reason_opt {
        format!("{base_message}: {reason}")
    } else if let Some(ctx) = full.revert_context.as_ref() {
        format!(
            "{base_message}: empty revert payload from top-level target {} (target_has_code={}, calldata_len={} bytes)",
            ctx.target, ctx.has_code, ctx.calldata_len
        )
    } else {
        format!("{base_message}: empty revert payload; no decoded reason")
    };

    if let Some(ctx) = failure_context.as_ref() {
        if let Some(extra) = format_failure_context(ctx) {
            message.push_str("; ");
            message.push_str(&extra);
        }
    }

    message
}

pub(super) async fn enrich_failure_reason_with_trace(
    simulator: &Arc<TxSimulator>,
    tx: &UnsignedTransaction,
    block_number: u64,
    base_message: &str,
    revert_reason: Option<&str>,
) -> String {
    let mut reason_opt = revert_reason.map(str::to_string);
    let mut failure_context: Option<FailureContext> = None;
    let needs_trace = reason_opt
        .as_deref()
        .map(|s| {
            let trimmed = s.trim();
            is_uninformative_revert(trimmed) || trimmed.contains("UniswapV2:")
        })
        .unwrap_or(true);

    if needs_trace {
        match simulator
            .simulate_unsigned_transaction_with_full_trace_at_block(tx.clone(), block_number)
            .await
        {
            Ok(full) => {
                failure_context = find_failure_context(&full.call_trace, 0);

                if let Some(reason) = extract_reason_from_call_trace(&full.call_trace)
                    .filter(|reason| !is_uninformative_revert(reason))
                {
                    reason_opt = Some(reason);
                } else if let Some(reason) = full
                    .revert_reason
                    .as_ref()
                    .filter(|reason| !is_uninformative_revert(reason))
                {
                    reason_opt = Some(reason.clone());
                } else if let Some(ctx) = failure_context.as_ref() {
                    if let Some(reason) = ctx
                        .reason
                        .clone()
                        .filter(|reason| !is_uninformative_revert(reason))
                    {
                        reason_opt = Some(reason);
                    }
                }

                if reason_opt
                    .as_deref()
                    .map(is_uninformative_revert)
                    .unwrap_or(true)
                {
                    if let Some(ctx) = full.revert_context.as_ref() {
                        reason_opt = Some(format!(
                            "execution reverted at {} (calldata {} bytes)",
                            ctx.target, ctx.calldata_len
                        ));
                    }
                }

                if reason_opt
                    .as_deref()
                    .map(is_uninformative_revert)
                    .unwrap_or(true)
                {
                    if let Some(ctx) = failure_context.as_ref() {
                        if let Some(reason) = ctx.reason.clone() {
                            if !reason.trim().is_empty() {
                                reason_opt = Some(reason);
                            }
                        }
                    }
                }
            }
            Err(err) => {
                let base = format_failure_with_revert(base_message, revert_reason);
                return format!("{base} (trace failed: {err})");
            }
        }
    }

    let base = format_failure_with_revert(base_message, reason_opt.as_deref());

    if let Some(ctx) = failure_context {
        if let Some(extra) = format_failure_context(&ctx) {
            return format!("{base}; {extra}");
        }
    }

    base
}

fn normalize_revert_text(reason: &str) -> String {
    reason
        .replace(
            "reverted without returning data",
            "returned an empty revert payload",
        )
        .replace(
            "Transaction reverted without data",
            "Empty revert payload from transaction execution",
        )
        .replace("Reverted without reason", "Empty revert payload")
}

fn is_uninformative_revert(reason: &str) -> bool {
    let trimmed = reason.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("execution reverted")
        || trimmed.eq_ignore_ascii_case("Reverted without reason")
        || trimmed.eq_ignore_ascii_case("Transaction reverted without data")
        || trimmed.eq_ignore_ascii_case("Empty revert payload")
        || trimmed.starts_with("Unknown error (0x")
        || trimmed.starts_with("unknown custom error selector 0x")
        || trimmed.contains("without returning data")
        || trimmed.contains("Empty revert payload from")
}

fn extract_reason_from_call_trace(frame: &CallFrame) -> Option<String> {
    if let Some(output) = frame.output.as_ref() {
        if let Some(reason) = describe_revert_output(output.as_ref()) {
            if !reason.is_empty() {
                return Some(reason);
            }
        }
    }

    let mut fallback: Option<String> = None;
    if let Some(error) = &frame.error {
        let trimmed = error.trim();
        if !trimmed.is_empty() && trimmed != "execution reverted" {
            if trimmed.contains("UniswapV2:") || trimmed.eq_ignore_ascii_case("execution reverted")
            {
                fallback = Some(trimmed.to_string());
            } else {
                if !is_uninformative_revert(trimmed) {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    for child in &frame.calls {
        if let Some(reason) = extract_reason_from_call_trace(child) {
            return Some(reason);
        }
    }

    fallback.or_else(|| {
        frame.error.as_ref().and_then(|err| {
            let trimmed = err.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    })
}

fn find_failure_context(frame: &CallFrame, depth: usize) -> Option<FailureContext> {
    for child in &frame.calls {
        if let Some(ctx) = find_failure_context(child, depth + 1) {
            return Some(ctx);
        }
    }

    let mut reason = frame
        .revert_reason
        .clone()
        .filter(|reason| !reason.trim().is_empty());

    if reason.is_none() {
        if let Some(output) = frame.output.as_ref() {
            reason = describe_revert_output(output.as_ref());
        }
    }

    if reason.is_none() {
        if let Some(error) = frame.error.as_ref() {
            let trimmed = error.trim();
            if !trimmed.is_empty() {
                reason = Some(trimmed.to_string());
            }
        }
    }

    if reason.is_some() || frame.to.is_some() {
        let selector = frame.selector();
        let decoded_call = decode_known_call(selector, frame.input.as_ref());
        return Some(FailureContext {
            reason,
            target: frame.to,
            selector,
            decoded_call,
            depth,
        });
    }

    None
}

fn decode_known_call(selector: Option<Selector>, input: &[u8]) -> Option<String> {
    let selector = selector?;
    let sel: [u8; 4] = selector.as_slice().try_into().ok()?;

    if input.len() < 4 {
        return None;
    }

    match sel {
        SELECTOR_TRANSFER_FROM => decode_transfer_from(input),
        SELECTOR_TRANSFER => decode_transfer(input),
        SELECTOR_APPROVE => decode_approve(input),
        SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS
        | SELECTOR_SWAP_EXACT_TOKENS_FOR_ETH_SUPPORTING_FEE
        | SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS_SUPPORTING_FEE => {
            decode_swap_exact_tokens_call(sel, input)
        }
        SELECTOR_GET_RESERVES => Some("getReserves()".to_string()),
        _ => None,
    }
}

fn decode_transfer_from(input: &[u8]) -> Option<String> {
    let from_word = read_word(input, 0)?;
    let to_word = read_word(input, 1)?;
    let value_word = read_word(input, 2)?;

    let from = decode_address(&from_word);
    let to = decode_address(&to_word);
    let amount = decode_u256(&value_word);

    Some(format!(
        "transferFrom(from={from:#x}, to={to:#x}, amount={amount})"
    ))
}

fn decode_transfer(input: &[u8]) -> Option<String> {
    let to_word = read_word(input, 0)?;
    let value_word = read_word(input, 1)?;

    let to = decode_address(&to_word);
    let amount = decode_u256(&value_word);

    Some(format!("transfer(to={to:#x}, amount={amount})"))
}

fn decode_approve(input: &[u8]) -> Option<String> {
    let spender_word = read_word(input, 0)?;
    let value_word = read_word(input, 1)?;

    let spender = decode_address(&spender_word);
    let amount = decode_u256(&value_word);

    Some(format!("approve(spender={spender:#x}, amount={amount})"))
}

fn decode_swap_exact_tokens_call(selector: [u8; 4], input: &[u8]) -> Option<String> {
    if input.len() < 4 + (5 * 32) {
        return None;
    }

    let amount_in_word = read_word(input, 0)?;
    let amount_out_min_word = read_word(input, 1)?;
    let path_offset_word = read_word(input, 2)?;
    let to_word = read_word(input, 3)?;
    let deadline_word = read_word(input, 4)?;

    let amount_in = decode_u256(&amount_in_word);
    let amount_out_min = decode_u256(&amount_out_min_word);
    let to_address = decode_address(&to_word);
    let deadline = word_to_u64(&deadline_word);

    let mut path_start = 4 + word_to_usize(&path_offset_word).unwrap_or(5 * 32);
    if path_start >= input.len() {
        path_start = 4 + (5 * 32);
    }
    if input.len() < path_start + 32 {
        return None;
    }

    let path_len_word: [u8; 32] = input[path_start..path_start + 32].try_into().ok()?;
    let path_len = word_to_usize(&path_len_word)?;
    let mut path = Vec::with_capacity(path_len);
    for i in 0..path_len {
        let start = path_start + 32 * (i + 1);
        if input.len() < start + 32 {
            return None;
        }
        let word: [u8; 32] = input[start..start + 32].try_into().ok()?;
        path.push(decode_address(&word));
    }

    let selector_name = if selector == SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS {
        "swapExactTokensForTokens"
    } else if selector == SELECTOR_SWAP_EXACT_TOKENS_FOR_ETH_SUPPORTING_FEE {
        "swapExactTokensForETHSupportingFeeOnTransferTokens"
    } else {
        "swapExactTokensForTokensSupportingFeeOnTransferTokens"
    };

    let path_str = path
        .iter()
        .map(|addr| format!("{addr:#x}"))
        .collect::<Vec<_>>()
        .join(" -> ");

    Some(format!(
        "{selector_name}(amountIn={amount_in}, amountOutMin={amount_out_min}, path=[{path_str}], to={to_address:#x}, deadline={deadline})"
    ))
}

fn read_word(input: &[u8], index: usize) -> Option<[u8; 32]> {
    let start = 4 + (index * 32);
    let end = start + 32;
    let slice = input.get(start..end)?;
    let mut word = [0u8; 32];
    word.copy_from_slice(slice);
    Some(word)
}

fn decode_address(word: &[u8; 32]) -> Address {
    Address::from_slice(&word[12..])
}

fn decode_u256(word: &[u8; 32]) -> U256 {
    U256::from_be_bytes(*word)
}

fn word_to_usize(word: &[u8; 32]) -> Option<usize> {
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&word[16..32]);
    let value = u128::from_be_bytes(bytes);
    usize::try_from(value).ok()
}

fn word_to_u64(word: &[u8; 32]) -> u64 {
    u64::from_be_bytes(word[24..32].try_into().unwrap_or([0u8; 8]))
}

fn format_failure_context(ctx: &FailureContext) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(addr) = ctx.target {
        parts.push(format!("call to {addr:#x}"));
    }

    if let Some(decoded) = ctx.decoded_call.as_ref() {
        parts.push(decoded.clone());
    } else if let Some(selector) = ctx.selector {
        parts.push(format!("selector 0x{}", hex::encode(selector.as_slice())));
    }

    if ctx.depth > 0 {
        parts.push(format!("depth {}", ctx.depth));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("failure originated from {}", parts.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, U256, address};
    use tx_simulator::RevertContext;

    fn frame(to: Address, input: Bytes, calls: Vec<CallFrame>) -> CallFrame {
        CallFrame {
            from: address!("0000000000000000000000000000000000000001"),
            gas: U256::from(1_000_000),
            gas_used: U256::from(10_000),
            to: Some(to),
            input,
            output: None,
            error: Some("execution reverted".to_string()),
            revert_reason: None,
            calls,
            logs: Vec::new(),
            value: Some(U256::ZERO),
            typ: "CALL".to_string(),
        }
    }

    fn word_u256(value: usize) -> [u8; 32] {
        let mut word = [0u8; 32];
        let value = value as u128;
        word[16..32].copy_from_slice(&value.to_be_bytes());
        word
    }

    fn word_address(address: Address) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[12..32].copy_from_slice(address.as_slice());
        word
    }

    fn word_bytes4(selector: [u8; 4]) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[0..4].copy_from_slice(&selector);
        word
    }

    fn dynamic_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&word_u256(bytes.len()));
        out.extend_from_slice(bytes);
        let padding = (32 - (bytes.len() % 32)) % 32;
        out.extend(std::iter::repeat(0).take(padding));
        out
    }

    fn error_string(message: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[0x08, 0xc3, 0x79, 0xa0]);
        out.extend_from_slice(&word_u256(32));
        out.extend_from_slice(&dynamic_bytes(message.as_bytes()));
        out
    }

    fn wrapped_error(target: Address, selector: [u8; 4], reason: &[u8], details: &[u8]) -> Vec<u8> {
        let reason_tail = dynamic_bytes(reason);
        let details_tail = dynamic_bytes(details);
        let details_offset = 4 * 32 + reason_tail.len();

        let mut out = Vec::new();
        out.extend_from_slice(&[0x90, 0xbf, 0xb8, 0x65]);
        out.extend_from_slice(&word_address(target));
        out.extend_from_slice(&word_bytes4(selector));
        out.extend_from_slice(&word_u256(4 * 32));
        out.extend_from_slice(&word_u256(details_offset));
        out.extend_from_slice(&reason_tail);
        out.extend_from_slice(&details_tail);
        out
    }

    #[test]
    fn full_trace_failure_replaces_empty_revert_wording() {
        let target = address!("C36442b4a4522E871399CD717aBDD847Ab11FE88");
        let child = frame(
            target,
            Bytes::from_static(&[0x88, 0x31, 0x64, 0x56]),
            Vec::new(),
        );
        let full = FullSimulationResult {
            success: false,
            gas_used: 100_000,
            revert_reason: Some(
                "Contract 0xC36442b4a4522E871399CD717aBDD847Ab11FE88 reverted without returning data"
                    .to_string(),
            ),
            revert_context: Some(RevertContext {
                target,
                has_code: true,
                calldata_len: 4,
            }),
            call_trace: frame(target, Bytes::from_static(&[0x88, 0x31, 0x64, 0x56]), vec![child]),
            struct_logs: None,
            logs: Vec::new(),
        };

        let message = format_failure_with_full_trace("Setup transaction replay failed", &full);

        assert!(message.contains("Setup transaction replay failed"));
        assert!(message.contains("empty revert payload"));
        assert!(message.contains("selector 0x88316456"));
        assert!(!message.contains("reverted without returning data"));
    }

    #[test]
    fn full_trace_decodes_universal_router_wrapped_error() {
        let target = address!("298a86cc43af878cb78ca20e80ab0de0a59a0444");
        let output = wrapped_error(
            target,
            [0x12, 0x34, 0x56, 0x78],
            &error_string("afterSwap rejected"),
            &[],
        );
        let full = FullSimulationResult {
            success: false,
            gas_used: 100_000,
            revert_reason: Some("Unknown error (0x90bfb865)".to_string()),
            revert_context: Some(RevertContext {
                target,
                has_code: true,
                calldata_len: 4,
            }),
            call_trace: CallFrame {
                from: address!("0000000000000000000000000000000000000001"),
                gas: U256::from(1_000_000),
                gas_used: U256::from(10_000),
                to: Some(target),
                input: Bytes::from_static(&[0x88, 0x31, 0x64, 0x56]),
                output: Some(Bytes::from(output)),
                error: Some("execution reverted".to_string()),
                revert_reason: None,
                calls: Vec::new(),
                logs: Vec::new(),
                value: Some(U256::ZERO),
                typ: "CALL".to_string(),
            },
            struct_logs: None,
            logs: Vec::new(),
        };

        let message =
            format_failure_with_full_trace("Universal Router V4 buy transaction failed", &full);

        assert!(message.contains("Universal Router V4 buy transaction failed"));
        assert!(message.contains("WrappedError"));
        assert!(message.contains("afterSwap rejected"));
        assert!(message.contains("0x12345678"));
        assert!(!message.contains("Unknown error (0x90bfb865)"));
    }
}
