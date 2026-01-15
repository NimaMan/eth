"""
DBReader class encapsulates all database read operations for the Sarigoz system. The DBReader leverages pre-defined SQL queries (stored in the queries folder) to answer common analytical questions such as:

    - Which addresses frequently interact with scam tokens?
    - Who are the top net winners or losers?
    - What are the detailed trade records for a given address?
    - How is the profit and loss (PNL) distributed across tokens?
    - What does the transaction relationship graph look like?
"""

from sqlalchemy import text
from typing import List, Any
from eth_data.database.eth_db_conn import get_db_session_maker

# Import queries from our queries modules:
from eth_data.database.queries.scam_queries import GET_ADDRESSES_SCAM_INTERACTIONS
from eth_data.database.queries.whale_queries import GET_TOP_NET_WINNERS, GET_TOP_NET_LOSERS
from eth_data.database.queries.graph_queries import GET_RELATIONSHIP_GRAPH
from eth_data.database.queries.trade_queries import GET_PNL_DISTRIBUTION_BY_TOKEN, GET_ADDRESS_PNL


class DBDataFetcher:
    def __init__(self):
        self.session = get_db_session_maker()
    
    def get_address_info(self, address: str) -> Any:
        """Retrieve detailed information for a given address."""
        query = text("SELECT * FROM pnl.addresses WHERE address = :address")
        result = self.session.execute(query, {"address": address})
        return result.fetchone()
    
    def get_token_info(self, token_address: str) -> Any:
        """Retrieve metadata for a given token."""
        query = text("SELECT * FROM pnl.tokens WHERE contract_address = :token_address")
        result = self.session.execute(query, {"token_address": token_address})
        return result.fetchone()
    
    def get_trades_for_address(self, address: str) -> List[Any]:
        """Retrieve all trade records for a specific address, ordered by entry_block."""
        query = text(GET_TRADES_FOR_ADDRESS)
        result = self.session.execute(query, {"address": address})
        return result.fetchall()
    
    def get_addresses_interacting_with_scam_tokens(self, limit: int = 100) -> List[Any]:
        """
        Retrieve addresses that frequently interact with scam tokens.
        Returns a list of addresses sorted by the number of scam-related interactions.
        """
        query = text(GET_ADDRESSES_SCAM_INTERACTIONS)
        result = self.session.execute(query, {"limit": limit})
        return result.fetchall()
    
    def get_top_net_winners(self, limit: int = 100) -> List[Any]:
        """
        Retrieve addresses with the highest total profit.
        """
        query = text(GET_TOP_NET_WINNERS)
        result = self.session.execute(query, {"limit": limit})
        return result.fetchall()
    
    def get_top_net_losers(self, limit: int = 100) -> List[Any]:
        """
        Retrieve addresses with the lowest total profit.
        """
        query = text(GET_TOP_NET_LOSERS)
        result = self.session.execute(query, {"limit": limit})
        return result.fetchall()
    
    def get_relationship_graph_data(self) -> List[Any]:
        """
        Retrieve relationship graph data for constructing the transaction network.
        """
        query = text(GET_RELATIONSHIP_GRAPH)
        result = self.session.execute(query)
        return result.fetchall()
    
    def get_pnl_distribution_by_token(self, token_address: str) -> List[Any]:
        """
        Retrieve trade PNL data for a given token, ordered by total profit.
        """
        query = text(GET_PNL_DISTRIBUTION_BY_TOKEN)
        result = self.session.execute(query, {"token_address": token_address})
        return result.fetchall()
    
    def get_address_pnl(self, address: str) -> Any:
        """
        Retrieve overall PNL metrics for a given address.
        """
        query = text(GET_ADDRESS_PNL)
        result = self.session.execute(query, {"address": address})
        return result.fetchone()
    
    def close(self):
        """Close the underlying database session."""
        self.session.close()


if __name__ == "__main__":
    db_reader = DBDataFetcher()
    # Example: Retrieve information for a specific address.
    addr_info = db_reader.get_address_info("0x2E027E2BE720B061c047EE50ff806916eDC6a9c3")
    print("Address Info:", addr_info)
    
    # Example: Retrieve the top 10 net winners.
    winners = db_reader.get_top_net_winners(limit=10)
    print("Top Net Winners:")
    for winner in winners:
        print(winner)
    
    # Close the DBReader session when finished.
    db_reader.close()