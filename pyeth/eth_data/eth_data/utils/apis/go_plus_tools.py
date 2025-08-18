import requests
from typing import Optional


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
        if contract_address.lower() in response['result']:
            return response['result'][contract_address.lower()]


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
