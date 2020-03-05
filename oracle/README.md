# Oracle module

### Overview

Oracle module can be used to feed the system with off-chain data. e.g. a bot can query off-chain data from other sources and submit it into on-chain storage by calling oracle module.

It uses a trait called `OperatorProvider::can_feed_data(who)` to ensure someone have permission to submit data otherwise it will fail with error `NoPermission`.
You can only submit updates once per block otherwise oracle will call `OnRedundantCall::multiple_calls_per_block(&who)` and fail with error `UpdateAlreadyDispatched`. The trait `OnRedundantCall` can be implemented to slash those who don't play by the roles, by default it does nothing.

Oracle module provides 2 public methods to submit data. Method `feed_value` is used to provide a single update and `feed_values` for batch updates. Keep in mind that you can call only one of them per block otherwise it will be a redundant call and fail. Both those methods are weight zero and dispatch class operational with a high priority, which means they can be misused to DoS the system, so be careful who you allow to call them and make sure to use `CheckOperator` to prevent multiple calls per block.

Module provides additional methods to read raw or combined data. `CombineData` provides a default implementation which finds a median from an array of valid values, but you can provide a custom implementation as well.
