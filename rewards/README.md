# Rewards module

This module exposes capabilities for staking rewards.

## Single asset algorithm

Consider a single pool with a single reward asset, generally, it will behave as next:

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
    # so until `rewards` grows, user will not be able to claim more than zero
    to_withdraw = inflate(pool, user_share)
    pool["rewards"] = pool["rewards"] + to_withdraw
    pool["withdrawn_rewards"] = pool["withdrawn_rewards"] + to_withdraw
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

### Prove

We want to prove that when a new share is added, it does not dilute previous rewards.

The user who adds a share after the reward is accumulated, will not get any part of the previous reward.

Let $R_n$ be the amount of the current reward asset.

Let $s_i$ be the stake of any specific user our of $m$ total users.

User current reward share equals

$$ r_i = R_n * ({s_i} / {\sum_{i=1}^m s_i}) $$

User $m + 1$ brings his share, so

$$r_i' = R_n * ({s_i} / {\sum_{i=1}^{m+1} s_i}) $$

$r_i > r_i'$, so the original share was diluted and a new user can claim the share of existing users.

What if we increase $R_n$ by $\delta_R$ so that original users get the same share.

We get:

$$ R_n * ({s_i} / {\sum_{i=1}^m s_i}) = ({R_n + \delta_R}) * ({s_i} / {\sum_{i=1}^{m+1} s_i})$$

After easy to do algebraic simplification we get

$$ \delta_R = R_n * ({s_m}/{\sum_{i=1}^{m} s_i}) $$

So for new share we increase reward pool. To compensate for that $\delta_R$ amount is marked as withdrawn from pool by new user.
