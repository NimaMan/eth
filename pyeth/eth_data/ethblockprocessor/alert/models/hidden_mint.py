import torch
from transformers import RobertaTokenizer, RobertaForSequenceClassification
import os
from functools import lru_cache


class HiddenMintPredictor:
    _instance = None
    _initialized = False
    
    def __new__(cls, *args, **kwargs):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self, model_path: str = None):
        """
        Initialize the HiddenMint predictor with a pre-trained model.
        
        Args:
            model_path: Path to the saved model weights. If None, looks for 'best_hidden_mint_model.pth'
                       in the same directory as this script.
        """
        if not HiddenMintPredictor._initialized:
            self.device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
            
            # Initialize tokenizer and model only once
            self.model_path = model_path or os.path.join(
                os.path.dirname(os.path.abspath(__file__)),
                'best_hidden_mint_model.pth'
            )
            self.tokenizer = RobertaTokenizer.from_pretrained("microsoft/codebert-base")
            self.model = RobertaForSequenceClassification.from_pretrained(
                "microsoft/codebert-base",
                num_labels=2
            ).to(self.device)
            
            # Load model weights with weights_only=True to avoid warning
            state_dict = torch.load(self.model_path, map_location=self.device, weights_only=True)
            self.model.load_state_dict(state_dict)
            self.model.eval()
            
            HiddenMintPredictor._initialized = True
        
        self.model.eval()
        
    @torch.no_grad()
    def predict(self, contract_code: bytes) -> dict:
        """
        Predict whether a contract might be a hidden mint.
        
        Args:
            contract_code: The bytecode of the contract (in bytes)
            
        Returns:
            Dictionary containing prediction and confidence score
        """
        # Convert bytecode to hex string
        if isinstance(contract_code, bytes):
            contract_code = contract_code.hex()
        
        # Ensure it starts with '0x'
        if not contract_code.startswith('0x'):
            contract_code = '0x' + contract_code
            
        # Tokenize the input
        encoded = self.tokenizer(
            contract_code,
            truncation=True,
            max_length=512,
            padding='max_length',
            return_tensors='pt'
        )
        
        # Move inputs to device
        input_ids = encoded['input_ids'].to(self.device)
        attention_mask = encoded['attention_mask'].to(self.device)
        
        # Get prediction
        outputs = self.model(input_ids, attention_mask=attention_mask)
        probabilities = torch.softmax(outputs.logits, dim=1)
        prediction = torch.argmax(outputs.logits, dim=1)
        
        return {
            'prediction': 'Hidden Mint' if prediction.item() == 1 else 'None',
            'hidden_mint_probability': probabilities[0][1].item()
        }
    
