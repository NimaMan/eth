
import pyreth

TOKEN_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"  # USDC
BLOCK_NUMBER = 17_000_000

chain_query = pyreth.PyReth().chain_query()
print("Trying with sealed header JSON from live publisher...")
meta_with_header = chain_query.get_token_metadata(
        TOKEN_ADDRESS, BLOCK_NUMBER, None
    )