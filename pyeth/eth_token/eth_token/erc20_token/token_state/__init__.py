"""Token state coordination package.

This namespace mirrors the responsibilities that currently live inside
``eth_token.erc20_token.data.erc20_token_data``.  It is intentionally
split into multiple helper modules so that the inevitable mechanical
move of the monolithic ``ERC20TokenData`` class can happen in digestible
steps.

Modules shipped in this package (see ``README.md`` for details):

``token_data`` – slim orchestration shell that will become the new home
    of ``ERC20TokenData`` once the helpers are wired in.
``control_address_tracker`` – code that keeps the creator/current owner
    set in sync, including Ownable2Step and AccessControl roles.
``token_transfer_tracker`` – ERC20/ETH transfer aggregation and
    per-address counters that power analytics.
``pool_state_bridge`` – glue between the token runtime and
    ``eth_token.erc20_token.pools`` (Uniswap V2/V3/V4) so pool logic has
    a single touch point.
``token_state_monitor`` – hidden-mint detection and contract-toggle
    tracking which still delegates to the existing ``token_health``
    package for advanced scoring.

At this stage the modules only contain documentation scaffolding; logic
will be copied over verbatim in a later change to guarantee zero
behavioural drift.
"""
