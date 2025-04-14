

class AddressTypeLabeler:
    def __init__(self, w3):
        self.w3 = w3

    def is_contract(self, address):
        # Normalize address to checksum format for consistency
        try:
            checksum_address = self.w3.to_checksum_address(address)
        except ValueError:
            raise ValueError(f"Invalid Ethereum address format: {address}")

        # Check if there's code at the address
        code = self.w3.eth.get_code(checksum_address)
        is_contract = len(code) > 2  # '0x' is returned for EOAs

        return is_contract

    def get_address_type(self, address):
        """Returns 'Contract' if the address is a contract, else 'Wallet'."""
        if self.is_contract(address):
            return "Contract"
        else:
            return "Wallet"
