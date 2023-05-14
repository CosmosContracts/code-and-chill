# CW721 Piggy Bank

[![Open in Gitpod](https://gitpod.io/button/open-in-gitpod.svg)](https://gitpod.io/#https://github.com/CosmosContracts/code-and-chill)

A Juno Code and Chill project! Supports NFTs that can hold deposits (like a piggy bank), deposits can be withdrawn if the NFT is burned.

- [x] Basic NFT contract (cw721)
- [x] Allow accounts to deposit funds (do we want to limit this?)
- [x] Override defaut burn method to payout funds in the piggy bank
- [x] Implement token balances query
- [ ] Refactor to support multiple types of tokens for deposits
- [ ] Tests!
- [ ] Bonus: return different token_uris if more money is in the piggy bank

Built with `cw721-base` from the [cw-nfts repo](https://github.com/cosmwasm/cw-nfts).

# Dynamic NFT functionality

There are a couple of approaches we can use to create dynamic NFTs:

- Simple token URI update from the minter or an approved account (most manual, would require indexer and bots for good UX, but works)
- Construct the metadata and images in the contract! (least gas efficient)
- Use a backend server (not very decentralized)
- Frontend code that handles the image generation based on on-chain metadata (may not always display correctly in wallets)
- Update token URI in the contract based on on-chain events

There are tradeoffs for all of these, but we are going to go for the last approach.

**Dynamic NFT Example: Trees**

A large part of handling this will be folder structure for all the metadata of all our NFTs!

Let's use the example of a tree that grows the more we feed it with carbon credits!

Here's a potential folder structure we could use:

```ignore
./metadata
  /1
    seedling.json
    sapling.json
    tree.json
    fullgrown.json
  /2
    seedling.json
    sapling.json
    tree.json
    fullgrown.json
```

The folders are `token_id` and inside those are all the possible states of each token. The entire metadata folder would be upload to IPFS, and NFTs would be minted with their initial seedling state as the initial `token_uri`.

As more funds are deposited in the NFT, we would have logic to update it's `token_uri` accordingly.

For example, when one token has been deposited, we update the `token_uri` to `<base_ipfs_hash>/<token_id>/sapling.json`, when ten tokens have been deposited we update it to `<base_ipfs_hash>/<token_id>/tree.json`, and when one hundred tokens have been deposited we update it to `<ipfs_hash>/<token_id>/fullgrown.json`.

One thing to note is that the folder structure of the metadata must be in-sync with the logic of the contract.
