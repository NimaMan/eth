import os
import pandas as pd
from dune_client.client import DuneClient # pip install dune-client


DUNE_API_KEY = os.environ.get("DUNE_API_KEY", '7xBqZKKPcJUzj2S6AkNcs4NktRTCcIhe')
ETH_DIR = os.environ.get("ETH_ADDRESS_DIR")
ETH_ADDRESS_DIR = os.path.join(ETH_DIR, "addresses")

dune = DuneClient(DUNE_API_KEY)
cex_addresses_query_result = dune.get_latest_result(3237025)
cex_addresses = pd.DataFrame(cex_addresses_query_result.result.rows)
cex_addresses.to_parquet(os.path.join(ETH_ADDRESS_DIR, "cex_addresses.parquet"))


query_result = dune.get_latest_result(2394100)
eth_stckaers = pd.DataFrame(query_result.result.rows)

# Convert columns to numeric, coercing errors (like 'Infinity') to NaN
numeric_cols = ['amount_staked', 'validators', 'marketshare', 'ow_change', 'om_change', 'sm_change', 'earned_rewards']
for col in numeric_cols:
    if col in eth_stckaers.columns:
        eth_stckaers[col] = pd.to_numeric(eth_stckaers[col], errors='coerce')

eth_stckaers.to_parquet(os.path.join(ETH_ADDRESS_DIR, "eth_stckaers.parquet"))


eth_etf_addresses = dune.get_latest_result(3749983)
eth_etf_addresses = pd.DataFrame(eth_etf_addresses.result.rows)
eth_etf_addresses.to_parquet(os.path.join(ETH_ADDRESS_DIR, "eth_etf_addresses.parquet"))

