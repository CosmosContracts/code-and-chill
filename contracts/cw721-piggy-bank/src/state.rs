use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Item, Map};

/// Map for storing NFT balances (token_id, amount)
pub const BALANCES: Map<&str, Uint128> = Map::new("nft_balances");

/// Denom used for depositing into piggy bank
pub const DEPOSIT_DENOM: Item<String> = Item::new("denoms");

/// The price to mint a new NFT
pub const MINT_PRICE: Item<Coin> = Item::new("mint_price");

/// The base url used for token metadata
pub const BASE_URL: Item<String> = Item::new("base_url");

/// Previous token id, represent the last NFT token ID that was minted
pub const PREVIOUS_TOKEN_ID: Item<u64> = Item::new("previous_token_id");

/// The max number of NFTs that can be minted
pub const MAX_NFT_SUPPLY: Item<u64> = Item::new("max_nft_supply");

/// The recipient for funds from initial NFT sale
pub const SALE_FUNDS_RECIPIENT: Item<Addr> = Item::new("sale_funds_recipient");
