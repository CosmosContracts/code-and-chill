use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CustomMsg, Empty, StdError, Uint128};
pub use cw721_base::{
    ContractError as BaseContractError, InstantiateMsg as BaseInstantiateMsg, MinterResponse,
};
use cw_storage_plus::{Item, Map};
use cw_utils::PaymentError;
use thiserror::Error;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-piggy-bank";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Implements extended on-chain metadata, by default cw721 NFTs only store a
// token_uri, which is a URL to off-chain metadata (same as ERC721).
#[cw_serde]
#[derive(Default)]
pub struct MetadataExt {
    // TODO support showing different token_uris based on how much is deposited
}

// This is the custom Execute message extension for this contract.
// Use it to implement custom functionality.
#[cw_serde]
pub enum ExecuteExt {
    /// Used to deposit funds in a particular NFT
    Deposit { token_id: String },
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
}

// The execute message type for this contract.
// If you don't need the Metadata and Execute extensions, you can use the
// `Empty` type.
pub type ExecuteMsg = cw721_base::ExecuteMsg<MetadataExt, ExecuteExt>;

// The query message type for this contract.
// If you don't need the QueryExt extension, you can use the
// `Empty` type.
pub type QueryMsg = cw721_base::QueryMsg<QueryExt>;

/// Custom errors for this contract
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    /// This inherits from cw721-base::ContractError to handle the base contract errors
    #[error("NFT contract error: {0}")]
    Cw721Error(#[from] cw721_base::ContractError),
}

/// Map for storing NFT balances (token_id, amount)
pub const BALANCES: Map<&str, Uint128> = Map::new("nft_balances");

pub const DENOM: Item<String> = Item::new("denoms");

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::{entry_point, to_binary, BankMsg};
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use cw_utils::must_pay;

    // This makes a conscious choice on the various generics used by the contract
    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // TODO Validate denoms are formated correctly

        // Save denoms
        DENOM.save(deps.storage, &msg.deposit_denom)?;

        // Instantiate the base contract
        Cw721Contract::default().instantiate(
            deps.branch(),
            env,
            info,
            BaseInstantiateMsg {
                minter: msg.minter,
                name: msg.name,
                symbol: msg.symbol,
            },
        )
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            // Optionally override the default cw721-base behavior
            ExecuteMsg::Burn { token_id } => execute_burn(deps, env, info, token_id),

            // Implment extension messages here, remove if you don't wish to use
            // An ExecuteExt extension
            ExecuteMsg::Extension { msg } => match msg {
                ExecuteExt::Deposit { token_id } => execute_deposit(deps, env, info, token_id),
            },

            // Use the default cw721-base implementation
            _ => Cw721Contract::default()
                .execute(deps, env, info, msg)
                .map_err(Into::into),
        }
    }

    pub fn execute_burn(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response, ContractError> {
        let base = Cw721Contract::default();

        let denom = DENOM.load(deps.storage)?;

        // Pay out the piggy bank!
        let balance = BALANCES.may_load(deps.storage, &token_id)?;
        let msgs = match balance {
            Some(balance) => {
                vec![BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: vec![Coin {
                        denom,
                        amount: balance,
                    }],
                }]
            }
            None => vec![],
        };

        // Pass off to default cw721 burn implementation, handles checking ownership
        base.execute(deps, env, info, ExecuteMsg::Burn { token_id })?;

        Ok(Response::default().add_messages(msgs))
    }

    pub fn execute_deposit(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response, ContractError> {
        // Check that funds were actually sent
        // TODO refactor to support multiple tokens as deposits
        let denom = DENOM.load(deps.storage)?;
        let amount = must_pay(&info, &denom)?;

        let base = Cw721Contract::default();

        // Check that the token exists
        base.tokens.load(deps.storage, &token_id)?;

        BALANCES.update(deps.storage, &token_id, |balance| -> StdResult<_> {
            // TODO BONUS maybe update NFT metadata?

            // TODO refactore to support multiple tokens as deposits
            Ok(balance.unwrap_or_default() + amount)
        })?;

        Ok(Response::default()
            .add_attribute("action", "deposit")
            .add_attribute("value", amount.to_string()))
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            // Optionally override a default cw721-base query
            // QueryMsg::Minter {} => unimplemented!(),
            QueryMsg::Extension { msg } => match msg {
                // Returns Coin type for the ballance of an NFT
                QueryExt::Balance { token_id } => to_binary(&Coin {
                    denom: DENOM.load(deps.storage)?,
                    amount: BALANCES
                        .may_load(deps.storage, &token_id)?
                        .unwrap_or_default(),
                }),
            },

            // Use default cw721-base query implementation
            _ => Cw721Contract::default().query(deps, env, msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        BankMsg, CosmosMsg,
    };

    /// Make sure cw2 version info is properly initialized during instantiation,
    /// and NOT overwritten by the base contract.
    #[test]
    fn proper_cw2_initialization() {
        let mut deps = mock_dependencies();

        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("larry", &[]),
            InstantiateMsg {
                name: "".into(),
                symbol: "".into(),
                minter: "larry".into(),
                deposit_denom: "ujuno".into(),
            },
        )
        .unwrap();

        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(version.contract, CONTRACT_NAME);
        assert_ne!(version.contract, cw721_base::CONTRACT_NAME);
    }

    #[test]
    fn happy_path() {
        let mut deps = mock_dependencies();
        const BOB: &str = "bob";

        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &[]),
            InstantiateMsg {
                name: "1337".into(),
                symbol: "1337".into(),
                minter: BOB.into(),
                deposit_denom: "ujuno".into(),
            },
        )
        .unwrap();

        // Mint the NFT
        entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &[]),
            ExecuteMsg::Mint {
                token_id: "1".into(),
                owner: BOB.into(),
                token_uri: Some("https://ipfs.io/cutedog.json".to_string()),
                extension: MetadataExt {},
            },
        )
        .unwrap();

        // Calling deposit funds without funds errors
        entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &[]),
            ExecuteMsg::Extension {
                msg: ExecuteExt::Deposit {
                    token_id: "1".to_string(),
                },
            },
        )
        .unwrap_err();

        // Calling deposit with wrong denom errors
        entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &coins(1000, "uatom")),
            ExecuteMsg::Extension {
                msg: ExecuteExt::Deposit {
                    token_id: "1".to_string(),
                },
            },
        )
        .unwrap_err();

        // Can't deposit to token id that doesn't exist
        let err = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &coins(1000, "ujuno")),
            ExecuteMsg::Extension {
                msg: ExecuteExt::Deposit {
                    token_id: "3".to_string(),
                },
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::Std(StdError::NotFound {
                kind: "cw721_base::state::TokenInfo<cw721_piggy_bank::MetadataExt>".to_string()
            })
        );

        // Calling deposit succeeds with correct token
        entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &coins(1000, "ujuno")),
            ExecuteMsg::Extension {
                msg: ExecuteExt::Deposit {
                    token_id: "1".to_string(),
                },
            },
        )
        .unwrap();

        // Only owner can burn NFT
        entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info("rando", &[]),
            ExecuteMsg::Burn {
                token_id: "1".to_string(),
            },
        )
        .unwrap_err();

        // Can't burn an NFT that doesn't exist
        let err = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &[]),
            ExecuteMsg::Burn {
                token_id: "2".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::Cw721Error(cw721_base::ContractError::Std(StdError::NotFound {
                kind: "cw721_base::state::TokenInfo<cw721_piggy_bank::MetadataExt>".to_string()
            }))
        );

        // Test burning NFT returns money
        let res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(BOB, &[]),
            ExecuteMsg::Burn {
                token_id: "1".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: BOB.to_string(),
                amount: coins(1000, "ujuno"),
            })
        );
    }
}
