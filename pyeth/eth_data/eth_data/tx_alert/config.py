# Minimal config to make imports work
# Alerts are commented out in live_block_processor.py so these aren't actually used

BRIBE_THRESHOLD_WEI = 10**17  # 0.1 ETH expressed in wei

def get_grey_addresses():
    return set()

def get_orca_addresses():
    return set()

def get_whale_addresses():
    return set()
