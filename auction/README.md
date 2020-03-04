# Auction module

### Overview

Auction module provides a way to open auction and place bids on-chain. You can open an auction by specifying a `start: BlockNumber` and/or an `end: BlockNumber`, and when the auction becomes active enabling anyone to place a bid at a higher price. Trait `AuctionHandler` is been used to validate the bid and when the auction ends `AuctionHandle::on_auction_ended(id, bid)` gets called.
