# Description: Test the HiddenMintPredict

from web3 import Web3
from ethblockprocessor.alert.models.hidden_mint import HiddenMintPredictor
from ethblockprocessor.alert.config import HIDDEN_MINT_MODEL_PATH


if __name__ == "__main__":
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    
    # Initialize predictor once (loads model)
    predictor = HiddenMintPredictor(HIDDEN_MINT_MODEL_PATH)
    
    # Get contract bytecode
    contract_address = "0x06fd1bAECcb6034F36226CB893bBA0Be4b0ad1a0" # incorrect
    contract_address = "0xA78877dCB65ad176c2632bb49C117dad2eff961a" # correct
    contract_code = w3.eth.get_code(contract_address)
    
    # Get prediction
    result = predictor.predict(contract_code)
    print(f"Prediction: {result['prediction']}")
    print(f"Probability: {result['probability']:.4f}")