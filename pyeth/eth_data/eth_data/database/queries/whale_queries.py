"""
Whale Queries Module
----------------------

This module contains SQL queries that help identify whale and large-holder behavior.
Key questions addressed:
    - Who are the top net winners (addresses with the highest total profit)?
    - Who are the top net losers (addresses with the lowest total profit)?
    - Which addresses hold large amounts of newly launched tokens?

Queries in this module:
    - GET_TOP_NET_WINNERS: Returns addresses sorted by descending total profit.
    - GET_TOP_NET_LOSERS: Returns addresses sorted by ascending total profit.
"""

GET_TOP_NET_WINNERS = """
SELECT * FROM pnl.addresses
ORDER BY total_profit DESC
LIMIT :limit
"""

GET_TOP_NET_LOSERS = """
SELECT * FROM pnl.addresses
ORDER BY total_profit ASC
LIMIT :limit
"""