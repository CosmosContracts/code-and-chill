use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
pub use cw721_base::{
    ContractError as BaseContractError, InstantiateMsg as BaseInstantiateMsg, MinterResponse,
};
use cw_utils::must_pay;

use crate::{
    msg::{Cw721Contract, ExecuteExt, ExecuteMsg, InstantiateMsg, QueryExt, QueryMsg},
    state::{BALANCES, DENOM},
    ContractError,
};

// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:cw721-piggy-bank";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// This makes a conscious choice on the various generics used by the contract
#[cfg_attr(not(feature = "library"), entry_point)]
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

#[cfg_attr(not(feature = "library"), entry_point)]
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
            ExecuteExt::UpdateTokenUri {
                token_id,
                token_uri,
            } => execute_update_token_uri(deps, env, info, token_id, token_uri),
        },

        // Use the default cw721-base implementation
        _ => Cw721Contract::default()
            .execute(deps, env, info, msg)
            .map_err(Into::into),
    }
}

pub fn execute_update_token_uri(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    token_uri: String,
) -> Result<Response, ContractError> {
    let base = Cw721Contract::default();

    // Check minter / admin to update token_uri
    let minter = base.minter(deps.as_ref())?;
    match minter.minter {
        Some(minter) => {
            if info.sender != minter {
                return Err(ContractError::Unauthorized {});
            }
        }
        None => {
            return Err(ContractError::Unauthorized {});
        }
    }

    // Update token_uri
    let mut token = base.tokens.load(deps.storage, &token_id)?;
    token.token_uri = Some(token_uri.clone());
    base.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::default()
        .add_attribute("action", "update_token_uri")
        .add_attribute("token_id", token_id)
        .add_attribute("token_uri", token_uri))
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
    let denom = DENOM.load(deps.storage)?;
    // Check the right kind of funds were sent
    let amount = must_pay(&info, &denom)?;

    let base = Cw721Contract::default();

    // Check that the token exists
    let mut token = base.tokens.load(deps.storage, &token_id)?;

    BALANCES.update(deps.storage, &token_id, |balance| -> StdResult<_> {
        let new_balance = balance.unwrap_or_default() + amount;

        // TODO don't hard code
        let base_url = "<insert_ipfs_url>";

        // Native token micro units are typically 6 decimal places
        // Check if balance is greater than 1
        if new_balance > Uint128::new(1000000) {
            token.token_uri = Some(format!("{}/{}/{}", base_url, token_id, "sapling.json"));
        } else if new_balance > Uint128::new(10000000) {
            token.token_uri = Some(format!("{}/{}/{}", base_url, token_id, "tree.json"));
        } else {
            token.token_uri = Some(format!("{}/{}/{}", base_url, token_id, "fullgrown.json"));
        }

        Ok(new_balance)
    })?;

    base.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::default()
        .add_attribute("action", "deposit")
        .add_attribute("value", amount.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
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
