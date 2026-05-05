import requests
from typing import Optional
from eth_token.erc20_token.pools.addresses import checksum_address


def get_url(chain_id: int) -> str:
    return f'https://api.gopluslabs.io/api/v1/token_security/{chain_id}'


def get_token_security(contract_address: str, chain_id: int=1) -> dict:
    """Retrieves token security information
    :param: Contract address
    :param: Chain ID (1 for Ethereum)
    :return: Dictionary
    """
    url = get_url(chain_id)
    params = {
        'contract_addresses': contract_address
    }
    response = requests.get(url, params=params)
    response = response.json()
    if 'result' in response:
        requested_address = checksum_address(contract_address)
        for result_address, result in response['result'].items():
            if checksum_address(result_address) == requested_address:
                return result


def is_honeypot(contract_address: str, chain_id: int=1) -> Optional[bool]:
    """Checks if the token is a honeypot, returns 1 if it is, 0 if it is not and None if it is unknown
    :param: Contract address
    :param: Chain ID (1 for Ethereum)
    :return: Optional[bool]
    """
    response = get_token_security(contract_address, chain_id)
    if response and 'is_honeypot' in response:
        return str_to_bool(response['is_honeypot'])


def str_to_bool(s: str) -> bool:
    return s.lower() == 'true'
