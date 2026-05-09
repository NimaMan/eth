use super::*;
use alloy_primitives::{Address, Bytes, Log as AlloyLog, B256, U256};

fn address(byte: u8) -> Address {
    Address::from([byte; 20])
}

fn topic_address(address: Address) -> B256 {
    let mut bytes = [0u8; 32];
    bytes[12..32].copy_from_slice(address.as_slice());
    B256::from(bytes)
}

fn word_u64(value: u64) -> Vec<u8> {
    let mut word = [0u8; 32];
    word[24..32].copy_from_slice(&value.to_be_bytes());
    word.to_vec()
}

#[test]
fn decodes_erc1155_transfer_batch() {
    let decoder = LogDecoder::new();
    let token = address(0x11);
    let operator = address(0x22);
    let from = address(0x33);
    let to = address(0x44);

    let mut data = Vec::new();
    data.extend(word_u64(64));
    data.extend(word_u64(160));
    data.extend(word_u64(2));
    data.extend(word_u64(7));
    data.extend(word_u64(8));
    data.extend(word_u64(2));
    data.extend(word_u64(11));
    data.extend(word_u64(12));

    let log = AlloyLog::new_unchecked(
        token,
        vec![
            decoder.signatures.transfer_batch,
            topic_address(operator),
            topic_address(from),
            topic_address(to),
        ],
        Bytes::from(data),
    );

    let decoded = decoder
        .decode_log(&log, 3)
        .expect("decode should succeed")
        .expect("event should decode");

    match decoded {
        DecodedEvent::ERC1155TransferEvent(event) => {
            assert_eq!(event.token_address, token);
            assert_eq!(event.operator, operator);
            assert_eq!(event.from_address, from);
            assert_eq!(event.to_address, to);
            assert_eq!(event.token_ids, vec![U256::from(7u64), U256::from(8u64)]);
            assert_eq!(event.amounts, vec![U256::from(11u64), U256::from(12u64)]);
            assert_eq!(event.log_index, 3);
        }
        other => panic!("unexpected decoded event: {:?}", other),
    }
}

#[test]
fn decodes_approval_for_all() {
    let decoder = LogDecoder::new();
    let token = address(0xaa);
    let owner = address(0xbb);
    let operator = address(0xcc);

    let log = AlloyLog::new_unchecked(
        token,
        vec![
            decoder.signatures.approval_for_all,
            topic_address(owner),
            topic_address(operator),
        ],
        Bytes::from(word_u64(1)),
    );

    let decoded = decoder
        .decode_log(&log, 9)
        .expect("decode should succeed")
        .expect("event should decode");

    match decoded {
        DecodedEvent::ApprovalForAllEvent(event) => {
            assert_eq!(event.token_address, token);
            assert_eq!(event.owner, owner);
            assert_eq!(event.operator, operator);
            assert!(event.approved);
            assert_eq!(event.log_index, 9);
        }
        other => panic!("unexpected decoded event: {:?}", other),
    }
}
