import requests
import time
from typing import Dict, Any, Optional


class EtherscanContractFetcher:
    def __init__(self, api_key: str, max_rate: int = 5, time_frame: int = 1):
        self.api_key = api_key
        self.base_url = "https://api.etherscan.io/api"
        self.max_rate = max_rate
        self.time_frame = time_frame
        self.call_times = []

    def _rate_limit(self):
        current_time = time.time()
        self.call_times = [t for t in self.call_times if current_time - t < self.time_frame]
        
        if len(self.call_times) >= self.max_rate:
            sleep_time = self.time_frame - (current_time - self.call_times[0])
            if sleep_time > 0:
                time.sleep(sleep_time)
        
        self.call_times.append(time.time())

    def fetch_contract_info(self, address: str, retries: int = 3) -> Optional[Dict[str, Any]]:
        params = {
            "module": "contract",
            "action": "getsourcecode",
            "address": address,
            "apikey": self.api_key
        }

        for attempt in range(retries):
            self._rate_limit()
            response = requests.get(self.base_url, params=params)
            
            if response.status_code == 200:
                data = response.json()
                if data["status"] == "1" and data["result"]:
                    return self._parse_response(data["result"][0])
                elif data["status"] == "0" and "Max rate limit reached" in data.get("result", ""):
                    print(f"Rate limit reached. Retrying in {self.time_frame} seconds...")
                    time.sleep(self.time_frame)
                else:
                    print(f"API returned an error: {data.get('message', 'Unknown error')}")
                    return None
            else:
                print(f"HTTP error occurred: {response.status_code}")
            
            if attempt < retries - 1:
                print(f"Retrying... (Attempt {attempt + 2} of {retries})")
            else:
                print("Max retries reached. Unable to fetch contract info.")
        
        return None

    def _parse_response(self, result: Dict[str, Any]) -> Dict[str, Any]:
        return {
            "contract_name": result.get("ContractName", ""),
            "source_code": result.get("SourceCode", ""),
            "compiler_version": result.get("CompilerVersion", ""),
            "optimization_used": result.get("OptimizationUsed", ""),
            "runs": result.get("Runs", ""),
            "constructor_arguments": result.get("ConstructorArguments", ""),
            "evm_version": result.get("EVMVersion", ""),
            "library": result.get("Library", ""),
            "license_type": result.get("LicenseType", ""),
            "proxy": result.get("Proxy", ""),
            "implementation": result.get("Implementation", ""),
            "swarm_source": result.get("SwarmSource", "")
        }

