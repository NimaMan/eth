"""
Known pool factory and router addresses for classification.

This helps identify which protocol a pool belongs to based on factory addresses.
"""

from web3 import Web3

# Factory addresses that create pools (checksummed)
POOL_FACTORIES = {
    # Uniswap V2
    'univ2_factory': Web3.to_checksum_address('0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f'),
    
    # Uniswap V3
    'univ3_factory': Web3.to_checksum_address('0x1F98431c8aD98523631AE4a59f267346ea31F984'),
    
    # Uniswap V4 (Pool Manager - singleton)
    'univ4_pool_manager': Web3.to_checksum_address('0x000000000004444C5DC75cB358380d2E3de08a90'),
    
    # SushiSwap
    'sushi_factory': Web3.to_checksum_address('0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac'),
    
    # PancakeSwap V2
    'pancake_factory': Web3.to_checksum_address('0xcA143Ce32Fe78f1f7019d7d551a6402fC5350C73'),
    
    # PancakeSwap V3
    'pancake_v3_factory': Web3.to_checksum_address('0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865'),
    
    # Curve Finance (Registry)
    'curve_registry': Web3.to_checksum_address('0x90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5'),
    'curve_factory': Web3.to_checksum_address('0xB9fC157394Af804a3578134A6585C0dc9cc990d4'),
    
    # Balancer V2 (Vault is the main contract)
    'balancer_vault': Web3.to_checksum_address('0xBA12222222228d8Ba445958a75a0704d566BF2C8'),
    
    # Kyber DMM (Dynamic Market Maker)
    'kyber_dmm_factory': Web3.to_checksum_address('0x833e4083B7ae46CeA85695c4f7ed25CDAd8886dE'),
    
    # DODO V2
    'dodo_v2_factory': Web3.to_checksum_address('0x3A97247DF274a17C59A3bd12735ea3FcDFb49950'),
    'dodo_dpp_factory': Web3.to_checksum_address('0x6B4Fa0bc61Eddc928e0Df9c7f01e407BfcD3e5EF'),  # Private pools
    
    # 1inch Liquidity Protocol
    'oneinch_v2_factory': Web3.to_checksum_address('0xbAF9A5d4b0052359326A6CDAb54BABAa3a3A9643'),
    
    # Shibaswap (Fork of SushiSwap)
    'shibaswap_factory': Web3.to_checksum_address('0x115934131916C8b277DD010Ee02de363c09d037c'),
    
    # Fraxswap (Fork of Uniswap V2 with TWAMM)
    'fraxswap_factory': Web3.to_checksum_address('0x43eC799eAdd63848443E2347C49f5f52e8Fe0F6f'),
}

# Router addresses (for reference) (checksummed)
ROUTERS = {
    'univ2_router': Web3.to_checksum_address('0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D'),
    'univ3_router': Web3.to_checksum_address('0xE592427A0AECe92De3Edee1F18E0157C05861564'),
    'univ3_router2': Web3.to_checksum_address('0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45'),
    'sushi_router': Web3.to_checksum_address('0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F'),
    'pancake_router': Web3.to_checksum_address('0xEfF92A263d31888d860bD50809A8D171709b7b1c'),
    'pancake_v3_router': Web3.to_checksum_address('0x13f4EA83D0bd40E75C8222255bc855a974568Dd4'),
    'kyber_router': Web3.to_checksum_address('0x1c87257F5e8609940Bc751a07BB085Bb7f8cDBE6'),
    'dodo_v2_router': Web3.to_checksum_address('0xa2398842F37465f89540430bDC00219fA9E4D28a'),
    'oneinch_router': Web3.to_checksum_address('0x1111111254EEB25477B68fb85Ed929f73A960582'),
    'shibaswap_router': Web3.to_checksum_address('0x03f7724180AA6b939894B5Ca4314783B0b36b329'),
    'fraxswap_router': Web3.to_checksum_address('0xC14d550632db8592D1243Edc8B95b0Ad06703867'),
}

def get_pool_protocol(factory_address: str) -> str:
    """
    Determine pool protocol based on factory address.
    
    Returns protocol name (univ2, univ3, univ4, sushi, etc.) or None.
    """
    # Compare with checksummed addresses
    if factory_address == POOL_FACTORIES['univ2_factory']:
        return 'univ2'
    elif factory_address == POOL_FACTORIES['univ3_factory']:
        return 'univ3'
    elif factory_address == POOL_FACTORIES['univ4_pool_manager']:
        return 'univ4'
    elif factory_address == POOL_FACTORIES['sushi_factory']:
        return 'sushi'
    elif factory_address == POOL_FACTORIES['pancake_factory']:
        return 'pancake'
    elif factory_address == POOL_FACTORIES['pancake_v3_factory']:
        return 'pancake_v3'
    elif factory_address in [POOL_FACTORIES['curve_registry'], POOL_FACTORIES['curve_factory']]:
        return 'curve'
    elif factory_address == POOL_FACTORIES['balancer_vault']:
        return 'balancer'
    elif factory_address == POOL_FACTORIES['kyber_dmm_factory']:
        return 'kyber'
    elif factory_address in [POOL_FACTORIES['dodo_v2_factory'], POOL_FACTORIES['dodo_dpp_factory']]:
        return 'dodo'
    elif factory_address == POOL_FACTORIES['oneinch_v2_factory']:
        return 'oneinch'
    elif factory_address == POOL_FACTORIES['shibaswap_factory']:
        return 'shibaswap'
    elif factory_address == POOL_FACTORIES['fraxswap_factory']:
        return 'fraxswap'
    
    return None

def is_v4_pool_manager(address: str) -> bool:
    """Check if address is the V4 Pool Manager."""
    return address == POOL_FACTORIES['univ4_pool_manager']

def is_known_factory(address: str) -> bool:
    """Check if address is a known pool factory."""
    return address in POOL_FACTORIES.values()

# Helper for identifying pool protocols when needed
KNOWN_POOL_PATTERNS = {
    # Can add known pool address patterns here if needed
    # For example, specific bytecode patterns or address ranges
}