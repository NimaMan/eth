/// Tests for event decoders

#[cfg(test)]
mod tests {
    use tx_processor::processing::{LogDecoder, DecodedEvent};
    use alloy_primitives::{Address, B256, U256, Log as AlloyLog, keccak256};
    use std::str::FromStr;

    #[test]
    fn test_erc20_transfer_decoder() {
        let decoder = LogDecoder::new();
        
        // Create an ERC20 transfer log
        let transfer_topic = keccak256(b"Transfer(address,address,uint256)");
        let from_address = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let to_address = Address::from_str("0x0987654321098765432109876543210987654321").unwrap();
        let amount = U256::from(1000u64);
        
        let topics = vec![
            transfer_topic,
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(from_address.as_slice());
                bytes
            }),
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(to_address.as_slice());
                bytes
            }),
        ];
        
        let data = amount.to_be_bytes_vec();
        let log = AlloyLog::new_unchecked(
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap(),
            topics,
            data.into(),
        );
        
        let result = decoder.decode_log(&log).unwrap();
        assert!(result.is_some());
        
        if let Some(DecodedEvent::ERC20Transfer(transfer)) = result {
            assert_eq!(transfer.from_address, from_address);
            assert_eq!(transfer.to_address, to_address);
            assert_eq!(transfer.amount, amount);
        } else {
            panic!("Expected ERC20Transfer event");
        }
    }

    #[test]
    fn test_erc721_transfer_decoder() {
        let decoder = LogDecoder::new();
        
        // Create an ERC721 transfer log (4 topics)
        let transfer_topic = keccak256(b"Transfer(address,address,uint256)");
        let from_address = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let to_address = Address::from_str("0x0987654321098765432109876543210987654321").unwrap();
        let token_id = U256::from(123u64);
        
        let topics = vec![
            transfer_topic,
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(from_address.as_slice());
                bytes
            }),
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(to_address.as_slice());
                bytes
            }),
            B256::from_slice(&token_id.to_be_bytes_vec()),
        ];
        
        let log = AlloyLog::new_unchecked(
            Address::from_str("0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D").unwrap(), // BAYC
            topics,
            vec![].into(),
        );
        
        let result = decoder.decode_log(&log).unwrap();
        assert!(result.is_some());
        
        if let Some(DecodedEvent::ERC721Transfer(transfer)) = result {
            assert_eq!(transfer.from_address, from_address);
            assert_eq!(transfer.to_address, to_address);
            assert_eq!(transfer.token_id, token_id);
        } else {
            panic!("Expected ERC721Transfer event");
        }
    }

    #[test]
    fn test_erc1155_transfer_decoder() {
        let decoder = LogDecoder::new();
        
        // Create an ERC1155 TransferSingle log
        let transfer_topic = keccak256(b"TransferSingle(address,address,address,uint256,uint256)");
        let operator = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let from_address = Address::from_str("0x0987654321098765432109876543210987654321").unwrap();
        let to_address = Address::from_str("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap();
        let token_id = U256::from(456u64);
        let amount = U256::from(100u64);
        
        let topics = vec![
            transfer_topic,
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(operator.as_slice());
                bytes
            }),
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(from_address.as_slice());
                bytes
            }),
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(to_address.as_slice());
                bytes
            }),
        ];
        
        let mut data = Vec::new();
        data.extend_from_slice(&token_id.to_be_bytes_vec());
        data.extend_from_slice(&amount.to_be_bytes_vec());
        
        let log = AlloyLog::new_unchecked(
            Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            topics,
            data.into(),
        );
        
        let result = decoder.decode_log(&log).unwrap();
        assert!(result.is_some());
        
        if let Some(DecodedEvent::ERC1155Transfer(transfer)) = result {
            assert_eq!(transfer.operator, operator);
            assert_eq!(transfer.from_address, from_address);
            assert_eq!(transfer.to_address, to_address);
            assert_eq!(transfer.token_ids, vec![token_id]);
            assert_eq!(transfer.amounts, vec![amount]);
        } else {
            panic!("Expected ERC1155Transfer event");
        }
    }

    #[test]
    fn test_uniswap_v3_initialize_decoder() {
        let decoder = LogDecoder::new();
        
        // Create a Uniswap V3 Initialize log
        let init_topic = keccak256(b"Initialize(uint160,int24)");
        let sqrt_price_x96 = U256::from(79228162514264337593543950336u128); // Example sqrt price
        let tick = -887220i32; // Example tick
        
        let topics = vec![init_topic];
        
        let mut data = Vec::new();
        data.extend_from_slice(&sqrt_price_x96.to_be_bytes_vec());
        // Convert tick to bytes (pad to 32 bytes)
        let tick_bytes = tick.to_be_bytes();
        let mut tick_32 = [0u8; 32];
        tick_32[28..].copy_from_slice(&tick_bytes);
        data.extend_from_slice(&tick_32);
        
        let log = AlloyLog::new_unchecked(
            Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            topics,
            data.into(),
        );
        
        let result = decoder.decode_log(&log).unwrap();
        assert!(result.is_some());
        
        if let Some(DecodedEvent::UniswapV3Initialize(init)) = result {
            assert_eq!(init.sqrt_price_x96, sqrt_price_x96);
            assert_eq!(init.tick, tick);
        } else {
            panic!("Expected UniswapV3Initialize event");
        }
    }

    #[test]
    fn test_topic_count_differentiation() {
        let decoder = LogDecoder::new();
        
        // Test that ERC20 and ERC721 transfers are differentiated by topic count
        let transfer_topic = keccak256(b"Transfer(address,address,uint256)");
        let from_address = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let to_address = Address::from_str("0x0987654321098765432109876543210987654321").unwrap();
        
        // ERC20 Transfer (3 topics)
        let erc20_topics = vec![
            transfer_topic,
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(from_address.as_slice());
                bytes
            }),
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(to_address.as_slice());
                bytes
            }),
        ];
        
        let erc20_log = AlloyLog::new_unchecked(
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap(),
            erc20_topics,
            U256::from(1000u64).to_be_bytes_vec().into(),
        );
        
        let erc20_result = decoder.decode_log(&erc20_log).unwrap();
        assert!(matches!(erc20_result, Some(DecodedEvent::ERC20Transfer(_))));
        
        // ERC721 Transfer (4 topics)
        let erc721_topics = vec![
            transfer_topic,
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(from_address.as_slice());
                bytes
            }),
            B256::from_slice(&{
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(to_address.as_slice());
                bytes
            }),
            B256::from_slice(&U256::from(123u64).to_be_bytes_vec()),
        ];
        
        let erc721_log = AlloyLog::new_unchecked(
            Address::from_str("0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D").unwrap(),
            erc721_topics,
            vec![].into(),
        );
        
        let erc721_result = decoder.decode_log(&erc721_log).unwrap();
        assert!(matches!(erc721_result, Some(DecodedEvent::ERC721Transfer(_))));
    }
}