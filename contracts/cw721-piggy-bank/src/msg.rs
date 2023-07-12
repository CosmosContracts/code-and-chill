use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CustomMsg, Empty};

// Implements extended on-chain metadata, by default cw721 NFTs only store a
// token_uri, which is a URL to off-chain metadata (same as ERC721).
#[cw_serde]
#[derive(Default)]
pub struct MetadataExt {}

// This is the custom Execute message extension for this contract.
// Use it to implement custom functionality.
#[cw_serde]
pub enum ExecuteExt {
    /// Used to deposit funds in a particular NFT
    Deposit {
        token_id: String,
    },
    UpdateTokenUri {
        token_id: String,
        token_uri: String,
    },
}
impl CustomMsg for ExecuteExt {}

// This is the custom Query message type for this contract.
// Use it to implement custom query messages.
#[cw_serde]
pub enum QueryExt {
    /// Query the current balance for an individual NFT
    Balance { token_id: String },
}
impl CustomMsg for QueryExt {}

// This contrains default cw721 logic with extensions.
// If you don't need a particular extension, replace it with an
// `Empty` type.
pub type Cw721Contract<'a> =
    cw721_base::Cw721Contract<'a, MetadataExt, Empty, ExecuteExt, QueryExt>;

#[cw_serde]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: String,

    /// Allowed denoms for deposit
    pub deposit_denom: String,

    /// Base URL represents the the base of the URI for token metadata
    /// Example: https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link/<token_id>
    pub base_url: String,

    /// The price to mint a new NFT
    pub mint_price: Coin,

    /// Max NFT supply, the maximum number of NFTs that can be minted
    /// If set None, unlimited NFTs can be minted
    pub max_nft_supply: Option<u64>,

    /// The recipient for funds raised from the NFT mint
    pub sale_funds_recipient: String,
}

// The execute message type for this contract.
// If you don't need the Metadata and Execute extensions, you can use the
// `Empty` type.
pub type ExecuteMsg = cw721_base::ExecuteMsg<MetadataExt, ExecuteExt>;

// The query message type for this contract.
// If you don't need the QueryExt extension, you can use the
// `Empty` type.
pub type QueryMsg = cw721_base::QueryMsg<QueryExt>;
