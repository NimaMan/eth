//! Utilities for determining the correct REVM spec based on block number

use revm_primitives::hardfork::SpecId;

/// Determine the correct REVM spec ID based on Ethereum mainnet block number
pub fn spec_id_from_block_number(block_number: u64) -> SpecId {
    match block_number {
        0..=1_149_999 => SpecId::FRONTIER,
        1_150_000..=1_919_999 => SpecId::HOMESTEAD,
        1_920_000..=2_462_999 => SpecId::DAO_FORK,
        2_463_000..=2_674_999 => SpecId::TANGERINE,
        2_675_000..=4_369_999 => SpecId::SPURIOUS_DRAGON,
        4_370_000..=7_279_999 => SpecId::BYZANTIUM,
        7_280_000..=9_068_999 => SpecId::CONSTANTINOPLE,
        9_069_000..=9_199_999 => SpecId::PETERSBURG,
        9_200_000..=12_243_999 => SpecId::ISTANBUL,
        12_244_000..=12_964_999 => SpecId::MUIR_GLACIER,
        12_965_000..=13_772_999 => SpecId::ARROW_GLACIER,
        13_773_000..=15_049_999 => SpecId::LONDON,
        15_050_000..=15_537_393 => SpecId::GRAY_GLACIER,
        15_537_394..=17_034_869 => SpecId::MERGE,
        17_034_870..=19_426_586 => SpecId::SHANGHAI,
        _ => SpecId::CANCUN, // 19_426_587+ is Cancun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_id_from_block_number() {
        // Test some key transitions
        assert_eq!(spec_id_from_block_number(0), SpecId::FRONTIER);
        assert_eq!(spec_id_from_block_number(1_150_000), SpecId::HOMESTEAD);
        assert_eq!(spec_id_from_block_number(12_965_000), SpecId::LONDON);
        assert_eq!(spec_id_from_block_number(15_537_394), SpecId::MERGE);
        assert_eq!(spec_id_from_block_number(17_034_870), SpecId::SHANGHAI);
        assert_eq!(spec_id_from_block_number(19_426_587), SpecId::CANCUN);
        assert_eq!(spec_id_from_block_number(22_646_153), SpecId::CANCUN); // Our test tx
    }
}