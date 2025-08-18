import asyncio
from aiohttp import ClientSession
from web3 import Web3
from pathlib import Path
from typing import List, Dict
import pandas as pd
from tqdm.asyncio import tqdm
import logging


RATE_LIMIT = 5  # 5 requests per second
BASE_URL = "https://api.etherscan.io/api"


# Set up logging
logging.basicConfig(filename='contract_processing.log', level=logging.INFO, 
                    format='%(asctime)s - %(levelname)s - %(message)s')


class SmartContractDataset:
    def __init__(self, base_path: str):
        self.data_path = Path(base_path)
        self.data_path.mkdir(parents=True, exist_ok=True)
        self.df = pd.DataFrame()

    def save_contract_data(self, contract_data: dict):
        new_df = pd.DataFrame([contract_data])
        self.df = pd.concat([self.df, new_df], ignore_index=True)

    def save_to_parquet(self):
        file_path = self.data_path / "contract_code_data.parquet"
        self.df.to_parquet(file_path, index=False)

    def load_from_parquet(self):
        file_path = self.data_path / "contract_code_data.parquet"
        if file_path.exists():
            self.df = pd.read_parquet(file_path)
        else:
            logging.info("No existing data file found.")

    def is_valid_contract_bytecode(self, contract_address: str) -> bool:
        contract_data = self.df[self.df["contract_address"] == contract_address]
        if contract_data.empty:
            return False
        return contract_data["bytecode"].iloc[0] is not None and contract_data["bytecode"].iloc[0] != "0x"
    
    def is_contract_data_available(self, contract_address: str) -> bool:
        # check if contract data has a valid bytecode or source code
        contract_data = self.df[self.df["contract_address"] == contract_address]
        if contract_data.empty:
            return False
        return contract_data["bytecode"].iloc[0] is not None or contract_data["source_code"].iloc[0] is not None

class RateLimiter:
    def __init__(self, rate_limit):
        self.rate_limit = rate_limit
        self.tokens = rate_limit
        self.updated_at = asyncio.get_event_loop().time()

    async def wait(self):
        while self.tokens <= 0:
            self.add_new_tokens()
            await asyncio.sleep(0.1)
        self.tokens -= 1

    def add_new_tokens(self):
        now = asyncio.get_event_loop().time()
        time_passed = now - self.updated_at
        new_tokens = time_passed * self.rate_limit
        if new_tokens > 1:
            self.tokens = min(self.tokens + new_tokens, self.rate_limit)
            self.updated_at = now


async def get_contract_bytecode_web3(contract_address: str, web3: Web3) -> str:
    try:
        bytecode = await web3.eth.get_code(Web3.to_checksum_address(contract_address))
        return bytecode.hex()
    except Exception as e:
        logging.error(f"Error fetching bytecode for contract {contract_address} using Web3: {str(e)}")
        return None


async def get_contract_bytecode_etherscan(session: ClientSession, contract_address: str, etherscan_api_key: str, rate_limiter: RateLimiter):
    await rate_limiter.wait()
    bytecode_params = {
        "module": "proxy",
        "action": "eth_getCode",
        "address": contract_address,
        "tag": "latest",
        "apikey": etherscan_api_key
    }
    async with session.get(BASE_URL, params=bytecode_params) as response:
        bytecode_data = await response.json()
    
    if bytecode_data.get("result") == 'Max rate limit reached':
        await asyncio.sleep(1)
        return await get_contract_bytecode_etherscan(session, contract_address, etherscan_api_key, rate_limiter)
    
    return bytecode_data.get("result")


async def get_contract_source_code_etherscan(session: ClientSession, contract_address: str, etherscan_api_key: str, rate_limiter: RateLimiter):
    await rate_limiter.wait()
    source_code_params = {
        "module": "contract",
        "action": "getsourcecode",
        "address": contract_address,
        "apikey": etherscan_api_key
    }
    async with session.get(BASE_URL, params=source_code_params) as response:
        source_code_data = await response.json()
    
    if source_code_data.get("result") == 'Max rate limit reached':
        await asyncio.sleep(1)
        return await get_contract_source_code_etherscan(session, contract_address, etherscan_api_key, rate_limiter)
    
    return source_code_data.get("result")[0].get("SourceCode") if source_code_data.get("status") == "1" else None


async def get_contract_byte_code(contract_address: str, etherscan_api_key: str, session: ClientSession, web3: Web3, rate_limiter: RateLimiter):
    bytecode = await get_contract_bytecode_etherscan(session, contract_address, etherscan_api_key, rate_limiter)
    
    if not bytecode or bytecode == "0x":
        bytecode = await get_contract_bytecode_web3(contract_address, web3)

    return contract_address, bytecode


async def fetch_contract_byte_code(contracts_to_process: List[str], etherscan_api_key: str, web3: Web3):
    rate_limiter = RateLimiter(RATE_LIMIT)
    results = []
    async with ClientSession() as session:
        tasks = [get_contract_byte_code(address, etherscan_api_key, session, web3, rate_limiter) for address in contracts_to_process]
        for future in tqdm(asyncio.as_completed(tasks), total=len(tasks)):
            result = await future
            results.append(result)
    return results


async def fetch_and_save_contract_source_and_byte_code(contract_address: str, etherscan_api_key: str, session: ClientSession, web3: Web3, rate_limiter: RateLimiter):
    bytecode = await get_contract_bytecode_etherscan(session, contract_address, etherscan_api_key, rate_limiter)
    
    if not bytecode or bytecode == "0x":
        bytecode = await get_contract_bytecode_web3(contract_address, web3)
    
    source_code = await get_contract_source_code_etherscan(session, contract_address, etherscan_api_key, rate_limiter)
    
    return contract_address, bytecode, source_code


