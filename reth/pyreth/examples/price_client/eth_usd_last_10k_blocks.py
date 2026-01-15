import pyreth

client = pyreth.PyReth().price_client()
rows = client.get_eth_usd_last_n_blocks(window_blocks=10_000, step_blocks=100, include_reserves=True)

print()