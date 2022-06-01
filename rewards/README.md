# Rewards module

### Overview

This module exposes capabilities for staking rewards.

If to consider single pool with single rewards assets, generally it will behave as next:

```python
from collections import defaultdict

pool = {}
pool["pool_total_shares"] = 0
pool["pool_rewards"] = 0
pool["pool_withdrawn_rewards"] = 0

users = defaultdict(lambda: dict(share = 0, withdrawn_reward = 0))

def add_share(pool, users, user, amount):
    old_total_shares = pool["pool_total_shares"]
    pool["pool_total_shares"] += amount
    inflation = 0 if old_total_shares == 0 else pool["pool_rewards"] * (amount / old_total_shares)
    pool["pool_rewards"] = pool["pool_rewards"] +  inflation
    pool["pool_withdrawn_rewards"] = pool["pool_withdrawn_rewards"] +  inflation
    user = users[user]
    user["share"] += amount
    user["withdrawn_reward"] += inflation

def accumulate_reward(pool, amount):
    pool["pool_rewards"] += amount

def claim_rewards(pool, users, user):
    user = users[user]
    reward_proportion = 0 if pool["pool_total_shares"] == 0 else  pool["pool_rewards"] * (user["share"] / pool["pool_total_shares"])
    to_withdraw = min(reward_proportion - user["withdrawn_reward"], pool["pool_rewards"] - pool["pool_withdrawn_rewards"])
    pool["pool_withdrawn_rewards"]  += to_withdraw
    user["withdrawn_reward"] += to_withdraw
    return to_withdraw
```
