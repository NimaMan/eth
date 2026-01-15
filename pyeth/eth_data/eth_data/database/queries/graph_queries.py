"""
Graph Queries Module
----------------------

This module contains SQL queries for constructing the transaction graph.
Key questions include:
    - What is the relationship network of addresses?
    - Who are the key nodes based on net flows?

Query in this module:
    - GET_RELATIONSHIP_GRAPH: Retrieves address pairs and their corresponding denom_flow values.
"""

GET_RELATIONSHIP_GRAPH = """
SELECT address, related_address, denom_flow
FROM pnl.related_addresses
"""