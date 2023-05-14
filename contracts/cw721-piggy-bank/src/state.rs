use cosmwasm_std::Uint128;
use cw_storage_plus::{Map, Item};

/// Map for storing NFT balances (token_id, amount)
pub const BALANCES: Map<&str, Uint128> = Map::new("nft_balances");

pub const DENOM: Item<String> = Item::new("denoms");