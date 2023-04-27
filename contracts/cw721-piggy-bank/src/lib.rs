use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CustomMsg, Empty, StdError, Uint128};
pub use cw721_base::{ContractError as BaseContractError, InstantiateMsg, MinterResponse};
use cw_storage_plus::Map;
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
    #[error("{0}")]
    Cw721Error(#[from] cw721_base::ContractError),
}

/// Map for storing NFT balances (token_id, amount)
/// TODO refactor to support multiple tokens as deposits? (maybe not needed)
/// TODO alteratively, leave as is and just allow reserve token to be configurable.
pub const BALANCES: Map<&str, Uint128> = Map::new("nft_balances");

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
        // TODO Maybe we customize the Instantiate message to include reserve tokens for piggy bank?
        // TODO Extend to support parameters for determining which image to show
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // Instantiate the base contract
        Cw721Contract::default().instantiate(deps.branch(), env, info, msg)
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

        // // Check ownership (not needed because checked in base, but for example)
        // let token = base.tokens.load(deps.storage, &token_id)?;
        // if info.sender != token.owner {
        //     return Err(ContractError::Ownership(
        //         cw721_base::OwnershipError::NotOwner,
        //     ));
        // }

        // Pay out the piggy bank!
        let balance = BALANCES.may_load(deps.storage, &token_id)?;
        let msgs = match balance {
            Some(balance) => {
                vec![BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: vec![Coin {
                        // TODO refactor to not hard code later
                        denom: "ujuno".to_string(),
                        amount: balance,
                    }],
                }]
            }
            None => vec![],
        };

        // Pass off to default cw721 burn implementation, handles checking ownership
        // TODO check ownership test (already handled in default implementation)
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
        let amount = must_pay(&info, "ujuno")?;

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
                // Returns Uint128
                QueryExt::Balance { token_id } => {
                    to_binary(&BALANCES.load(deps.storage, &token_id)?)
                }
            },

            // Use default cw721-base query implementation
            _ => Cw721Contract::default().query(deps, env, msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

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
            },
        )
        .unwrap();

        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(version.contract, CONTRACT_NAME);
        assert_ne!(version.contract, cw721_base::CONTRACT_NAME);
    }
}
