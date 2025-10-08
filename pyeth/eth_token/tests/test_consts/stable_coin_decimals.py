from eth_data.chain_utils.common_addresses import (
    STABLECOINS_ADDRESS_BY_NAME,
    ERC20_TOKEN_DECIMALS,
)
from eth_data.address.contract_type import get_erc20_contract_info
from web3 import Web3


w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
assert w3.is_connected(), "Web3 provider not connected"

mismatches = []

for symbol, address in STABLECOINS_ADDRESS_BY_NAME.items():
    info = get_erc20_contract_info(address, w3=w3)
    if info is None:
        mismatches.append((symbol, "Not an ERC20 token"))
        continue

    expected = ERC20_TOKEN_DECIMALS.get(symbol)
    actual = info.get("decimals")
    if expected != actual:
        mismatches.append((symbol, f"Expected {expected}, got {actual}"))

if mismatches:
    print("Mismatched tokens:")
    for token, issue in mismatches:
        print(f"{token}: {issue}")
    raise AssertionError(f"{len(mismatches)} mismatches found")

print("All stablecoin decimal values matched.")
