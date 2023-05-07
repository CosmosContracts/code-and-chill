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
