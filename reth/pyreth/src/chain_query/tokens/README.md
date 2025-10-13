Token Metadata Bindings
=======================

Purpose
-------
- Group token-related helpers for the ChainQuery Python bindings, keeping the main binding surface tidy.

What’s exposed to Python (via pyreth.ChainQuery)
------------------------------------------------
- get_token_decimals(token: str, block: Optional[int]) -> int
- get_token_symbol(token: str, block: Optional[int]) -> str
- get_token_name(token: str, block: Optional[int]) -> str
- get_token_total_supply(token: str, block: Optional[int]) -> str (U256 as decimal string)
- get_token_metadata(token: str, block: Optional[int]) -> TokenMetadata

Data class
----------
- TokenMetadata(address: str, name: str, symbol: str, decimals: int, total_supply: str)

Notes
-----
- Methods are implemented in chain_query.rs to avoid privacy issues accessing the internal provider/runtime.
- This folder documents and organizes token bindings; future refactors can split code while keeping a clean layout.
