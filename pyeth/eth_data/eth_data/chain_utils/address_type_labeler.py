

try:
    import pyreth
except ImportError as exc:  # pragma: no cover
    raise ImportError("pyreth must be installed to use AddressTypeLabeler") from exc


class AddressTypeLabeler:
    def __init__(self, w3=None):
        self._pyreth = pyreth.PyReth()
        self._chain_query = self._pyreth.chain_query()

    def is_contract(self, address):
        return self._chain_query.is_contract(address, None)

    def get_address_type(self, address):
        """Returns 'Contract' if the address is a contract, else 'Wallet'."""
        return "Contract" if self.is_contract(address) else "Wallet"
