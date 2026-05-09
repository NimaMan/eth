use alloy_primitives::U256;

pub(super) fn read_abi_offset(data: &[u8], offset: usize) -> Option<usize> {
    let word = data.get(offset..offset + 32)?;
    if word[..24].iter().any(|byte| *byte != 0) {
        return None;
    }
    usize::try_from(u64::from_be_bytes(word[24..32].try_into().ok()?)).ok()
}

pub(super) fn read_u256_array(data: &[u8], offset: usize) -> Option<Vec<U256>> {
    let len = read_abi_offset(data, offset)?;
    let mut values = Vec::with_capacity(len);
    let start = offset.checked_add(32)?;
    for index in 0..len {
        let word_start = start.checked_add(index.checked_mul(32)?)?;
        let word = data.get(word_start..word_start + 32)?;
        values.push(U256::from_be_slice(word));
    }
    Some(values)
}
