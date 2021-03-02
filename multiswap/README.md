# Multiswap

This is a contract that contains many token swap pools.
Each pool can have up to 10 tokens and it's own fee %.

## Usage

- deposit funds / withdraw funds of the contract's virtual balance
- create a pool with specific set of tokens and a fee, get `pool_id`
- add liquidity to specific pool from the funds deposited
- remove liquidity from specific pool back into deposited funds on the contract
- anyone can swap with any pool by using `ft_transfer_call` with msg == `swap:<pool_id>:<token_out>:<min_token_amount>` 
