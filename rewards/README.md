# Rewards module

### Overview

This module exposes capabilities for staking rewards.

### Single asset algorithm

If to consider single pool with single reward asset, generally it will behave as next:

```python
from collections import defaultdict

pool = {}
pool["shares"] = 0
pool["rewards"] = 0
pool["withdrawn_rewards"] = 0

users = defaultdict(lambda: dict(shares = 0, withdrawn_rewards = 0))

def inflate(pool, user_share):
    return 0 if pool["shares"] == 0 else pool["rewards"] * (user_share / pool["shares"])

def add_share(pool, users, user, user_share):
    # virtually we add more rewards, but claim they were claimed by user
    # so until `rewards` grows, users will not be able to claim more than zero
    to_withdraw = inflate(pool, user_share)
    pool["rewards"] = pool["rewards"] +  to_withdraw
    pool["withdrawn_rewards"] = pool["withdrawn_rewards"] +  to_withdraw
    pool["shares"] += user_share
    user = users[user]
    user["shares"] += user_share
    user["withdrawn_rewards"] += to_withdraw

def accumulate_reward(pool, amount):
    pool["rewards"] += amount

def claim_rewards(pool, users, user):
    user = users[user]
    inflation = inflate(pool, user["shares"])
    to_withdraw = min(inflation - user["withdrawn_rewards"], pool["rewards"] - pool["withdrawn_rewards"])
    pool["withdrawn_rewards"]  += to_withdraw
    user["withdrawn_rewards"] += to_withdraw
    return to_withdraw
```
