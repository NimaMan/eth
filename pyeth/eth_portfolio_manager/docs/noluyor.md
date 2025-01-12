# PNL 
One thing to remeber here is that the scam is logarithmic.  

- Categorize the addresses into different categories:
    - The ones that keep the same address for a long time
    - The ones that change address frequently
    - The ones that have a lot of trades 
        - The wales
        - The orcas 


- The scam labeler needs correction

# eth_token_monitor

- Add txn info such as the hash to the live token. currently we miss that in the bribe alert. 
- Add api to be able to get the current live tokens
- Add more alerts
- A preslae case 



## Alerts

## False positives:

- Predicted as scam not might be a false positive:

1. 2024-12-30 09:36:49,488 - INFO - SCAM ALERT: Scam Txn: 0x0ee168976e21b22aa9e64b2d97081d4c245e4aace0bbf9560302338757d84a26 Token: 0x8ABB7c18D713a477D345D92fcfCCF26C1d971009 Reason: {'Malicious actors involved'} Confidence: 0.95 Involved Addresses: ['0xC36095006a9c31329577c80FE94a5316e30411c6', '0x6a806Ae02B39F65fC99f9F6b603a2dCA4cC9df2b', '0x75E8E7Baa73c95af2B9623D6A37a09EA356c4327']



2. "2024-12-30 10:09:49,660 - INFO - SCAM ALERT: Scam Txn: 0xc44e6f8098f2b349d264c69e5aa3d59ca8cd52b2b8a45d23975691d1136cd670 Token: 0x71b00242B52338dB008D7007F6937cdd34D40319 Reason: {'Malicious actors involved'} Confidence: 0.95 Involved Addresses: ['0xC36095006a9c31329577c80FE94a5316e30411c6', '0x6a806Ae02B39F65fC99f9F6b603a2dCA4cC9df2b', '0x75E8E7Baa73c95af2B9623D6A37a09EA356c4327']


# Alert Performance Calculator 
- Jelle can write this one:
    - input: token that is being updated 
    - output: how much? 






# What do we need to finish the loop?
    - BlockProcessor: The only thing that is missing is the uniswap v3 positions. 
    - TokenProcessor: The only thing that is missing is the uniswap v3 positions. 
    
    - PotfolioManager: This has a basic backtest in place. 
    - TxnManager: This is missing. write down the code. 
        - start with a simple one on a strategy that you know will work for sure 
        - submit txn 