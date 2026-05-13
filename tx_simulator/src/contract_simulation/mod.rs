/// Contract simulation helpers.
///
/// This module contains read-only contract method simulation plus small ABI
/// helpers used by examples and higher-level query crates.
pub mod calldata;
pub mod output;
pub mod read_only;

pub use calldata::{encode_contract_read_call_no_args, encode_contract_read_call_with_address_arg};
pub use output::{
    decode_string_from_contract_output, decode_uint256_from_contract_output,
    decode_uint8_from_contract_output,
};
