# open-runtime-module-library

[![Crates.io](https://img.shields.io/crates/v/orml-tokens)](https://crates.io/search?q=orml)
[![codecov](https://codecov.io/gh/open-web3-stack/open-runtime-module-library/branch/master/graph/badge.svg?token=FZ4HZYMW9A)](https://codecov.io/gh/open-web3-stack/open-runtime-module-library)
[![GitHub](https://img.shields.io/github/license/open-web3-stack/open-runtime-module-library)](https://github.com/open-web3-stack/open-runtime-module-library/blob/master/LICENSE)

The Open Runtime Module Library (ORML) is a community maintained collection of Substrate runtime modules.

## Runtime Modules Overview

#### Utility
- [auction](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/auction)
	- Implements a generalized auction interface, used by Acala for liquidation auctions.
- [authority](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/authority)
	- Allow more advanced permission configuration such as timelock for governance actions.
- [gradually-update](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/gradually-update)
	- Provides way to adjust numeric parameter gradually over a period of time.
- [oracle](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/oracle)
	- Allow offchain oracle providers to feed data to be consumed by onchain pallets.
- [rewards](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/rewards)
	- Implements ability to calculate and distribute token staking rewards.
- [traits](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/traits)
	- Implements various utility traits including BasicCurrency, MultiCurrency, Auction and more. Used by other ORML pallets.

#### Tokens
- [asset-registry](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/asset-registry)
	- Register asset / token metadata including name, decimals, and XCM MultiLocation
	- Partially based on the Acala’s asset-registry pallet, which includes some Acala specific code (e.g. EVM+) so not suitable for other teams.
- [currencies](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/currencies)
	- Provide an unified interface to combine pallet-balances and orml-tokens
- [nft](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/nft)
	- Provide a non-fungible-token implementation
- [payments](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/payments)
	- This pallet allows users to create secure reversible payments that keep funds locked in a merchant’s account until the off-chain goods are confirmed to be received. Each payment gets assigned its own judge that can help resolve any disputes between the two parties.
- [tokens](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/tokens)
	- Implements fungible tokens pallet with compatibility with Substrate tokens abstractions
- [vesting](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/vesting)
	- Provides scheduled balance locking mechanism, in a *graded vesting* way.

#### XCM
- [xcm-support](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/xcm-support)
	- Provides supporting traits, types and implementations, to support cross-chain message(XCM) integration with ORML modules.
- [xcm](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/xcm)
	- Provides a way for governance body to dispatch XCM.
- [xtokens](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/xtokens)
	- Provide crosschain token transfer functionality.
	- Used by multiple parachains for their XCM token transfer implementation.

#### Benchmarking
- [benchmarking](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/benchmarking)
	- Fork of frame-benchmarking in Substrate to allow implement runtime specific benchmarks

## Example

Checkout [orml-workshop](https://github.com/xlc/orml-workshop) for example usage.

## Development

### Makefile targets

- `make check`
	- Type check the code, without std feature, excluding tests.
- `make check-tests`
	- Type check the code, with std feature, including tests.
- `make test`
	- Run tests.

### `Cargo.toml`

ORML use `Cargo.dev.toml` to avoid workspace conflicts with project cargo config. To use cargo commands in ORML workspace, create `Cargo.toml` by running

- `cp Cargo.dev.toml Cargo.toml`, or
- `make Cargo.toml`, or
- change the command to `make dev-check` etc which does the copy. (For the full list of `make` commands, check `Makefile`)

# Web3 Foundation Grant Project
ORML is part of the bigger `Open-Web3-Stack` initiative, that is currently under a General Grant from Web3 Foundation. See Application details [here](https://github.com/open-web3-stack/General-Grants-Program/blob/master/grants/speculative/open_web3_stack.md). The 1st milestone has been delivered.

# Projects using ORML
- [If you intend or are using ORML, please add your project here](https://github.com/open-web3-stack/open-runtime-module-library/edit/master/README.md)

_In alphabetical order_

- [Acala Network](https://github.com/AcalaNetwork/Acala)
- [Ajuna Network](https://github.com/ajuna-network/Ajuna)
- [Astar Network](https://github.com/AstarNetwork)
- [Bifrost Finance](https://github.com/bifrost-finance/bifrost)
- [Bit.Country](https://github.com/bit-country/Bit-Country-Blockchain)
- [Centrifuge](https://github.com/centrifuge/centrifuge-chain)
- [ChainX](https://github.com/chainx-org/ChainX)
- [Composable](https://github.com/ComposableFi/composable)
- [Crust](https://github.com/crustio/crust)
- [GameDAO Protocol](https://github.com/gamedaoco)
- [HydraDX](https://github.com/galacticcouncil/hack.HydraDX-node)
- [Interlay and Kintsugi](https://github.com/interlay/interbtc)
- [InvArch and Tinkernet](https://github.com/InvArch/InvArch-Node)
- [KodaDot: MetaPrime Network](https://github.com/kodadot/metaprime.network)
- [Laminar Chain](https://github.com/laminar-protocol/laminar-chain)
- [Libra](https://github.com/atscaletech/libra)
- [Listen](https://github.com/listenofficial)
- [Manta Network](https://github.com/Manta-Network)
- [Mangata Finance](https://github.com/mangata-finance)
- [Minterest](https://github.com/minterest-finance/minterest-chain-node)
- [Moonbeam](https://github.com/PureStake/moonbeam/)
- [OAK](https://github.com/OAK-Foundation/OAK-blockchain)
- [Parallel Finance](https://github.com/parallel-finance/)
- [PolkaFoundry Network](https://github.com/PolkaFoundry)
- [Setheum Network](https://github.com/Setheum-Labs/Setheum)
- [Titan Network](https://github.com/titan-foundation/titan)
- [Valiu Liquidity Network](https://github.com/valibre-org/vln-node)
- [Zeitgeist](https://github.com/zeitgeistpm/zeitgeist)
- [ZERO Network](https://github.com/playzero/subzero)

