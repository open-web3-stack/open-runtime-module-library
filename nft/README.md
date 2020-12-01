# Non-fungible-token module

### Overview

Non-fungible-token module provides basic functions to create and manager NFT(non fungible token) such as `create_class`, `transfer`, `mint`, `burn`, `destroy_class`.

- `create_class` create NFT(non fungible token) class
- `transfer` transfer NFT(non fungible token) to another account.
- `mint` mint NFT(non fungible token)
- `burn` burn NFT(non fungible token)
- `destroy_class` destroy NFT(non fungible token) class

### Integration Testing

Integration tests are written in JavaScript (TypeScript) using the Polkadot JS API and should be executed on a local dev network that contains this NFT pallet. The following integration tests are needed to cover most of functionality and corner cases of the NFT pallet:

#### Create Class
##### Normal use case
##### Zero size collection metadata
##### 1 byte collection metadata
##### 1 KB collection metadata 
##### 1 MB collection metadata
##### 1 GB collection metadata

#### Transfers
##### Normal Alice to Bob tranfer
##### Repeated normal transfer Alice -> Bob -> Alice -> Bob
##### Normal Alice to Alice transfer
##### Transfer unowned token Alice -> Bob
##### Transfer unowned token Alice -> Alice
##### Double transfer Alice -> Bob, Alice -> Bob
##### Transfer non-existing token of existing class
##### Transfer token of non-existing class

#### Minting
##### Normal minting 
##### 0 byte token metadata
##### 1 byte token metadata
##### 1 KB token metadata
##### 1 MB token metadata
##### 1 GB token metadata
##### Non-existent class ID
##### Private minting: Class owner is allowed to mint
##### Private minting: Non-privileged address is not allowed to mint
##### SetMintMode can make minting public
##### SetMintMode can make minting private
##### SetMintMode with invalid class ID

#### Burning
##### Normal burn
##### Burn a token of a non-existing class 
##### Burn a non-existing token of existing class
##### Burn a non-owned token

#### Destroy Class
##### Normal destruction
##### Destroy a class with non-zero total issuance
##### Destroy a non-owned class
##### Destroy a non-existing class

#### Stress Testing
##### Create 1, 10, 100 ... up to 1e6 classes
##### Create 1, 10, 100 ... up to 1e7 tokens in a class
##### Test is_owner when 1e7 tokens are created
##### Create 1e7 tokens with 1MB metadata each
