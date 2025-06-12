# Decode Events Testing Documentation

## Overview
This document defines the comprehensive test suite for the Rust event decoding module in `tx_processor`. These tests validate correct parsing of Ethereum log events from real mainnet transactions, ensuring compatibility with the existing Python implementation.

## Important Notes
- All topic hashes have been verified to match the keccak256 hash of their event signatures
- Event parsing logic matches the Python implementation in `eth_block_processor/txn/txn_log_processor.py`
- Some events share the same signature (e.g., ERC20 and ERC721 Transfer) and require contract-level differentiation
- Test data uses real mainnet addresses and realistic values where possible

## Test Categories

### 1. Uniswap V2 Events

#### 1.1 Sync Event
- **Event Signature**: `Sync(uint112,uint112)`
- **Topic Hash**: `0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1`
- **Test Data**:
  - Pair Address: `0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0`
  - Reserve0: `249011873154970419771` (from hex: `0x0d7fbbe35020a4023b`)
  - Reserve1: `25099878520000000000` (from hex: `0x15c54ae87720eb000`)
  - Log Index: 136
- **Validation**: Ensure reserves are decoded as strings to handle uint256 precision

#### 1.2 Swap Event
- **Event Signature**: `Swap(address,uint256,uint256,uint256,uint256,address)`
- **Topic Hash**: `0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822`
- **Test Data**:
  - Pair Address: `0x52fd104d206ee67f5d26bb66f64781af887b2eb0`
  - Sender: `0x7d217fFC52898191D97eF38EF448c8F46Ce880e2`
  - To: `0x7d217fFC52898191D97eF38EF448c8F46Ce880e2`
  - Amount0In: `619486577000000000`
  - Amount1In: `0`
  - Amount0Out: `0`
  - Amount1Out: `62410893000000000`
  - Log Index: 137
- **Validation**: Verify all amounts are strings, addresses are checksummed

#### 1.3 PairCreated Event
- **Event Signature**: `PairCreated(address,address,address,uint256)`
- **Topic Hash**: `0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9`
- **Test Data**:
  - Factory: `0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f`
  - Token0: `0x2551Bc3f26129019624F1fB09ebB880E94dBc22A`
  - Token1: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
  - Pair Address: `0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0`
  - Pair Number: 402846 (from hex: `0x6259e`)
  - Log Index: 273
- **Validation**: Ensure address extraction from topics and data

#### 1.4 Mint Event
- **Event Signature**: `Mint(address,uint256,uint256)`
- **Topic Hash**: `0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f`
- **Test Data**:
  - Pair Address: `0x52fd104d206ee67f5d26bb66f64781af887b2eb0`
  - Sender: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
  - Amount0: `250000000000000000000`
  - Amount1: `25000000000000000000`
  - Log Index: 281
- **Validation**: Amounts as strings, proper address formatting

### 2. ERC20 Events

#### 2.1 Transfer Event
- **Event Signature**: `Transfer(address,address,uint256)`
- **Topic Hash**: `0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef`
- **Test Data**:
  - Token: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` (USDC)
  - From: `0x5B3b5DF2BF2B6543f78e053bD91C4Bdd820929f1`
  - To: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
  - Amount: `1000000000` (1000 USDC - 6 decimals)
  - Log Index: 100
- **Validation**: Amount as string, addresses checksummed

#### 2.2 Approval Event
- **Event Signature**: `Approval(address,address,uint256)`
- **Topic Hash**: `0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925`
- **Test Data**:
  - Token: `0x2551Bc3f26129019624F1fB09ebB880E94dBc22A`
  - Owner: `0xa51B1C4766F0fC9409525784b3BF62385dDfA08d`
  - Spender: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
  - Amount: `115792089237316195423570985008687907853269984665640564039207584007913129639935` (max uint256)
  - Log Index: 275
- **Validation**: Handle max uint256 value correctly as string

#### 2.2 Transfer Event
- **Event Signature**: `Transfer(address,address,uint256)`
- **Topic Hash**: `0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef`
- **Note**: While not in the provided tests, this is critical for token tracking
- **Validation**: Similar to Approval, ensure proper amount handling

### 3. WETH Events

#### 3.1 Deposit Event
- **Event Signature**: `Deposit(address,uint256)`
- **Topic Hash**: `0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c`
- **Test Data**:
  - WETH Address: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
  - Sender: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
  - Amount: `25000000000000000000` (25 ETH)
  - Log Index: 276
- **Validation**: Amount as string, address checksumming

### 4. Uniswap V3 Events

#### 4.1 Initialize Event
- **Event Signature**: `Initialize(uint160,int24)`
- **Topic Hash**: `0x98636036cb66a9c19a37435efc1e90142190214e8abeb821bdba3f2990dd4c95`
- **Test Data**:
  - Pool Address: `0xbe931909a485643ACdb7EeF4184c1284cd8eFe8f`
  - SqrtPriceX96: `79625380684338992030278`
  - Tick: `-276224`
  - Log Index: 200
- **Validation**: Handle large uint160 values, signed int24 ticks

#### 4.2 Swap Event
- **Event Signature**: `Swap(address,address,int256,int256,uint160,uint128,int24)`
- **Topic Hash**: `0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67`
- **Test Data**:
  - Pool Address: `0x7C706586679Af2BA6D1A9fC2DA9C6aF59883fdD3`
  - Sender: `0x51C72848c68a965f66FA7a88855F9f7784502a7F`
  - Recipient: `0x51C72848c68a965f66FA7a88855F9f7784502a7F`
  - Amount0: `1144687188178682368`
  - Amount1: `-477257693222` (negative value)
  - SqrtPriceX96: `51200387744091159339590900`
  - Liquidity: `548981905163348089`
  - Tick: `-146895`
  - Log Index: 66
- **Validation**: Handle signed integers (int256), large uint values

### 5. Uniswap V4 Events

#### 5.1 Initialize Event
- **Event Signature**: Custom V4 signature
- **Topic Hash**: `0xdd466e674ea557f56295e2d0218a125ea4b4f0f6f3307b95f85e6110838d6438`
- **Test Data**:
  - Pool Manager: `0x000000000004444c5dc75cB358380D2e3dE08A90`
  - Event ID: `0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3`
  - Currency0: `0x0000000000000000000000000000000000000000` (ETH)
  - Currency1: `0x51ea52a6A885DD34f30494768cf1D9DC9f004185`
  - Fee: 3000
  - Tick Spacing: 60
  - Hooks: `0x0000000000000000000000000000000000000000`
  - SqrtPriceX96: `13041953694121149609898616840968`
  - Tick: 102077
  - Log Index: 44
- **Validation**: Complex data extraction from multiple topics

#### 5.2 ModifyLiquidity Event
- **Event Signature**: Custom V4 signature
- **Topic Hash**: `0xf208f4912782fd25c7f114ca3723a2d5dd6f3bcc3ac8db5af63baa85f711d5ec`
- **Test Data**:
  - Pool Manager: `0x000000000004444c5dc75cB358380D2e3dE08A90`
  - Event ID: `0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3`
  - Sender: `0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e`
  - Tick Lower: `-887220`
  - Tick Upper: `887220`
  - Liquidity Delta: `2452727715443591093860`
  - Salt: `0x00000000000000000000000000000000000000000000000000000000000010af`
  - Log Index: 47
- **Validation**: Signed tick values, large liquidity amounts

#### 5.3 Swap Event
- **Event Signature**: Custom V4 signature
- **Topic Hash**: `0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f`
- **Test Data**:
  - Pool Manager: `0x000000000004444c5dc75cB358380D2e3dE08A90`
  - Event ID: `0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3`
  - Sender: `0x66a9893cC07D91D95644AEDD05D03f95e1dBA8Af`
  - Amount0: `-86740761572630000` (negative)
  - Amount1: `2328746925604862476941`
  - SqrtPriceX96: `12963585779093724829272953188292`
  - Liquidity: `2452727715443591093860`
  - Tick: `101956`
  - Fee: `3000`
  - Log Index: 165
- **Validation**: Negative amounts, very large numbers

### 6. Permit2 Event
- **Event Signature**: Custom Permit2 signature
- **Topic Hash**: `0xc6a377bfc4eb120024a8ac08eef205be16b817020812c73223e81d1bdb9708ec`
- **Test Data**:
  - Permit2 Contract: `0x000000000022D473030F116dDEE9F6B43aC78BA3`
  - Owner: `0x84f607C948951B7Bfe3EdaEFA47E192Ca4D6a6aA`
  - Token: `0x51ea52a6A885DD34f30494768cf1D9DC9f004185`
  - Spender: `0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e`
  - Amount: `1461501637330902918203684832716283019655932542975` (max uint256)
  - Expiration: `1742990094`
  - Nonce: `0`
  - Log Index: 45
- **Validation**: Handle permit signatures and expiration timestamps

## Test Implementation Requirements

### 1. Data Types
- All amounts must be represented as strings to preserve precision
- Addresses must be checksummed (EIP-55)
- Support for signed integers (int24, int256)
- Support for unsigned integers up to uint256

### 2. Hex Decoding
- Proper handling of `0x` prefixed hex strings
- Big-endian byte ordering
- Zero-padding for shorter values

### 3. Topic Parsing
- Topic[0] is always the event signature hash
- Indexed parameters appear in topics[1], topics[2], etc.
- Non-indexed parameters are ABI-encoded in the data field

### 4. Error Handling
- Invalid topic count
- Malformed hex data
- Overflow/underflow for signed integers
- Invalid addresses (not 20 bytes)

### 5. Performance Considerations
- Use efficient hex decoding libraries
- Avoid unnecessary allocations
- Consider batch processing for multiple events

## Additional Events Missing Test Coverage

### 7. ERC721 Events

#### 7.1 ERC721 Transfer Event
- **Event Signature**: `Transfer(address,address,uint256)`
- **Topic Hash**: `0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef` (same as ERC20)
- **Test Data**:
  - NFT Contract: `0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D` (BAYC)
  - From: `0x0000000000000000000000000000000000000000` (mint)
  - To: `0xb47e3cd837dDF8e4c57F05d70Ab865de6e193BBB`
  - Token ID: `1234`
- **Validation**: Same event signature as ERC20 Transfer - differentiation must be done at higher level by checking contract type or token ID patterns

#### 7.2 ERC721 Approval Event
- **Event Signature**: `Approval(address,address,uint256)`
- **Topic Hash**: `0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925`
- **Test Data**:
  - NFT Contract: `0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D`
  - Owner: `0xb47e3cd837dDF8e4c57F05d70Ab865de6e193BBB`
  - Approved: `0x1E0049783F008A0085193E00003D00cd54003c71` (OpenSea)
  - Token ID: `1234`
- **Validation**: Token ID instead of amount

### 8. ERC1155 Events

#### 8.1 TransferSingle Event
- **Event Signature**: `TransferSingle(address,address,address,uint256,uint256)`
- **Topic Hash**: `0xc3d58168c5ae7397731d063d5bbf3d657854427343f4c083240f7aacaa2d0f62`
- **Test Data**:
  - Contract: `0x76BE3b62873462d92a7880a91B6F706d72d1b36C`
  - Operator: `0x1E0049783F008A0085193E00003D00cd54003c71`
  - From: `0x5b3256965e7C3cF26E11FCAf296DfC8807C01073`
  - To: `0x8c90E6CAa2b5a0E8776bE5c9a69dC8D99BaB5Aae`
  - Token ID: `10001`
  - Amount: `5`
- **Validation**: Both token ID and amount

#### 8.2 TransferBatch Event
- **Event Signature**: `TransferBatch(address,address,address,uint256[],uint256[])`
- **Topic Hash**: `0x4a39dc06d4c0dbc64b70af90fd698a233a518aa5d07e595d983b8c0526c8f7fb`
- **Test Data**:
  - Contract: `0x76BE3b62873462d92a7880a91B6F706d72d1b36C`
  - Operator: `0x1E0049783F008A0085193E00003D00cd54003c71`
  - From: `0x5b3256965e7C3cF26E11FCAf296DfC8807C01073`
  - To: `0x8c90E6CAa2b5a0E8776bE5c9a69dC8D99BaB5Aae`
  - Token IDs: `[10001, 10002, 10003]`
  - Amounts: `[5, 10, 1]`
- **Validation**: Array decoding for both IDs and amounts

### 9. Token Control Events

#### 9.1 OwnershipTransferred Event
- **Event Signature**: `OwnershipTransferred(address,address)`
- **Topic Hash**: `0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0`
- **Test Data**:
  - Token Contract: `0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE` (SHIB)
  - Previous Owner: `0x0000000000000000000000000000000000000000`
  - New Owner: `0xb8e4526e0181eb6358B6451C7102270C8e5Ea84c`
  - Log Index: 10
- **Validation**: Zero address for renounced ownership

#### 9.2 TradingEnabled Event
- **Event Signature**: `TradingEnabled(uint256)`
- **Topic Hash**: `0x7b0a47d3b0234280b6c9213c5bbff44c8b6001bea7770b3950280f91410532d6`
- **Test Data**:
  - Token Contract: `0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE`
  - Block Number: `18500000` (encoded in data field)
  - Log Index: 15
- **Validation**: Block number from data field

#### 9.3 TradingDisabled Event
- **Event Signature**: `TradingDisabled(uint256)`
- **Topic Hash**: `0x691f4eac2b8850491851c72f70a121d76b20836d776658438f5b13dd9f8dbc6e`
- **Test Data**:
  - Token Contract: `0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE`
  - Block Number: `18600000` (encoded in data field)
  - Log Index: 20
- **Validation**: Block number from data field

### 10. Uniswap V2 Additional Events

#### 10.1 Burn Event
- **Event Signature**: `Burn(address,uint256,uint256,address)`
- **Topic Hash**: `0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496`
- **Test Data**:
  - Pair Address: `0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc` (USDC/ETH)
  - Sender: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D` (from topic[1])
  - Amount0: `1000000000` (1000 USDC)
  - Amount1: `500000000000000000` (0.5 ETH)
  - To: `0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f` (from topic[2])
  - Log Index: 150
- **Validation**: Sender and To from topics, amounts from data

#### 10.2 Withdrawal Event (WETH)
- **Event Signature**: `Withdrawal(address,uint256)`
- **Topic Hash**: `0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65`
- **Note**: While this event exists in WETH contract, it's NOT currently parsed in the Python implementation
- **Test Data**:
  - WETH Address: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
  - Src: `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`
  - Amount: `1000000000000000000` (1 ETH)
  - Log Index: 200
- **Validation**: ETH unwrapping amount (implement if needed for completeness)

### 11. Uniswap V3 Additional Events

#### 11.1 PoolCreated Event
- **Event Signature**: `PoolCreated(address,address,uint24,int24,address)`
- **Topic Hash**: `0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118`
- **Test Data**:
  - Factory: `0x1F98431c8aD98523631AE4a59f267346ea31F984`
  - Token0: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` (USDC)
  - Token1: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` (WETH)
  - Fee: `3000` (0.3%)
  - Tick Spacing: `60`
  - Pool: `0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8`
- **Validation**: Fee tier and tick spacing

#### 11.2 Mint Event (V3)
- **Event Signature**: `Mint(address,address,int24,int24,uint128,uint256,uint256)`
- **Topic Hash**: `0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde`
- **Test Data**:
  - Pool: `0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8`
  - Owner: `0x5B3b5DF2BF2B6543f78e053bD91C4Bdd820929f1`
  - Tick Lower: `-887220`
  - Tick Upper: `887220`
  - Liquidity: `1000000000000000000`
  - Amount0: `1000000000` (1000 USDC)
  - Amount1: `500000000000000000` (0.5 ETH)
- **Validation**: Tick range and liquidity

#### 11.3 Burn Event (V3)
- **Event Signature**: `Burn(address,int24,int24,uint128,uint256,uint256)`
- **Topic Hash**: `0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c`
- **Test Data**:
  - Pool: `0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8`
  - Owner: `0x5B3b5DF2BF2B6543f78e053bD91C4Bdd820929f1` (from topic[1])
  - Tick Lower: `-887220` (from topic[2])
  - Tick Upper: `887220` (from topic[3])
  - Liquidity: `500000000000000000`
  - Amount0: `500000000` (500 USDC)
  - Amount1: `250000000000000000` (0.25 ETH)
- **Validation**: Owner and ticks from topics, amounts from data

#### 11.4 IncreaseLiquidity Event
- **Event Signature**: `IncreaseLiquidity(uint256,uint128,uint256,uint256)`
- **Topic Hash**: `0x3067048beee31b25b2f1681f88dac838c8bba36af25bfb2b7cf7473a5847e35f`
- **Test Data**:
  - Position Manager: `0xC36442b4a4522E871399CD717aBDD847Ab11FE88`
  - Token ID: `123456`
  - Liquidity: `2000000000000000000`
  - Amount0: `2000000000` (2000 USDC)
  - Amount1: `1000000000000000000` (1 ETH)
- **Validation**: NFT position tracking

#### 11.5 DecreaseLiquidity Event
- **Event Signature**: `DecreaseLiquidity(uint256,uint128,uint256,uint256)`
- **Topic Hash**: `0x26f6a048ee9138f2c0ce266f322cb99228e8d619ae2bff30c67f8dcf9d2377b4`
- **Test Data**:
  - Position Manager: `0xC36442b4a4522E871399CD717aBDD847Ab11FE88`
  - Token ID: `123456`
  - Liquidity: `1000000000000000000`
  - Amount0: `1000000000` (1000 USDC)
  - Amount1: `500000000000000000` (0.5 ETH)
- **Validation**: Partial position removal

### 12. Generic Event Handling

#### 12.1 Unknown Event
- **Test Case**: Event with unrecognized topic hash
- **Test Data**:
  - Contract: `0x1234567890123456789012345678901234567890`
  - Topic Hash: `0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890`
  - Topics: Variable count
  - Data: Variable length
- **Validation**: Graceful handling, store raw data for analysis

## Test Structure

Each test should:
1. Define the raw log data (address, topics, data)
2. Call the appropriate decode function
3. Validate all fields match expected values
4. Check proper type conversions (strings for amounts)
5. Verify address checksumming

## Integration Tests

Beyond unit tests, create integration tests that:
1. Process complete transaction receipts
2. Handle multiple events in correct order
3. Validate event relationships (e.g., Mint before Swap)
4. Test error recovery for malformed events

## Benchmarks

Include benchmarks for:
1. Single event decoding
2. Batch event processing (100, 1000, 10000 events)
3. Memory allocation patterns
4. Comparison with Python implementation speed