use std::fmt::format;

use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use cw_denom::UncheckedDenom;
pub use cw721_base::{
    ContractError as BaseContractError, InstantiateMsg as BaseInstantiateMsg, MinterResponse,
};
use cw_utils::must_pay;
use url::Url;

use crate::{
    msg::{Cw721Contract, ExecuteExt, ExecuteMsg, InstantiateMsg, MetadataExt, QueryExt, QueryMsg},
    state::{
        BALANCES, BASE_URL, DEPOSIT_DENOM, MAX_NFT_SUPPLY, MINT_PRICE, PREVIOUS_TOKEN_ID,
        SALE_FUNDS_RECIPIENT,
    },
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

    // Validate denoms are formatted correctly
    let unchecked_denom = UncheckedDenom::Native(msg.deposit_denom.clone());
    let _checked_denom = unchecked_denom.into_checked(deps.as_ref()).map_err(|_| StdError::generic_err("Invalid deposit denom"))?;
    
    // Save config info
    DEPOSIT_DENOM.save(deps.storage, &_checked_denom.to_string())?;
    // validate base_url is a real url
    let _parsed_url = Url::parse(&msg.base_url).map_err(|_| StdError::generic_err("Invalid base URL"))?;
    BASE_URL.save(deps.storage, &msg.base_url)?;
    MINT_PRICE.save(deps.storage, &msg.mint_price)?;
    if let Some(max_nft_supply) = msg.max_nft_supply {
        MAX_NFT_SUPPLY.save(deps.storage, &max_nft_supply)?;
    }
    SALE_FUNDS_RECIPIENT.save(
        deps.storage,
        &deps.api.addr_validate(&msg.sale_funds_recipient)?,
    )?;

    // Set initial previous token id to zero
    PREVIOUS_TOKEN_ID.save(deps.storage, &0)?;

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

        // Overrides default Mint method. Used to purchase and create initial NFTs
        ExecuteMsg::Mint { .. } => execute_mint(deps, env, info),

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

    let denom = DEPOSIT_DENOM.load(deps.storage)?;

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

// NOTE: in real life, add some randomness to minting, otherwise people will game it
pub fn execute_mint(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Load mint_price and base_url
    let base_url = BASE_URL.load(deps.storage)?;
    let mint_price = MINT_PRICE.load(deps.storage)?;

    // Check the right amount of funds were sent
    let amount = must_pay(&info, &mint_price.denom)?;
    if amount != mint_price.amount {
        return Err(ContractError::WrongAmount {});
    }

    // Load previous_token_id, incrementing it, saving the new value, and returning the result
    let next_token_id = PREVIOUS_TOKEN_ID.update(deps.storage, |previous_id| {
        Ok::<u64, StdError>(previous_id + 1)
    })?;

    // Check the collection hasn't been minted out
    let max_supply = MAX_NFT_SUPPLY.may_load(deps.storage)?;
    if let Some(max_supply) = max_supply {
        if next_token_id > max_supply {
            return Err(ContractError::MintedOut {});
        }
    }

    // Pay out funds to creator or recipient of sale funds
    let recipient = SALE_FUNDS_RECIPIENT.load(deps.storage)?;
    let msg = BankMsg::Send {
        to_address: recipient.to_string(),
        amount: info.funds.clone(),
    };

    // Mint the NFT and assign to the sender
    let base = Cw721Contract::default();
    base.execute(
        deps,
        env,
        info.clone(),
        ExecuteMsg::Mint {
            token_id: next_token_id.to_string(),
            owner: info.sender.to_string(),
            // Formats the NFT token_uri (which links to the metadata) based on the token_id and the initial
            // state for the dynamic NFT (in this case NFT trees start out as seedlings)
            token_uri: Some(format!(
                "{}/{}/{}",
                base_url,
                next_token_id.to_string(),
                "seedling.json"
            )),
            extension: MetadataExt {},
        },
    )?;

    Ok(Response::default().add_message(msg))
}

pub fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // Check that funds were actually sent
    let denom = DEPOSIT_DENOM.load(deps.storage)?;
    // Check the right kind of funds were sent
    let amount = must_pay(&info, &denom)?;

    let base = Cw721Contract::default();

    // Load base URL for token metadata
    let base_url = BASE_URL.load(deps.storage)?;

    // Check that the token exists
    let mut token = base.tokens.load(deps.storage, &token_id)?;

    BALANCES.update(deps.storage, &token_id, |balance| -> StdResult<_> {
        let new_balance = balance.unwrap_or_default() + amount;

        // Native token micro units are typically 6 decimal places
        // Check if balance is greater than 1
        if new_balance > Uint128::new(1000000) {
            token.token_uri = Some(format!("{}/{}/{}", base_url, token_id, "sapling.json"));
        } else if new_balance > Uint128::new(10000000) {
            token.token_uri = Some(format!("{}/{}/{}", base_url, token_id, "tree.json"));
        } else if new_balance > Uint128::new(100000000) {
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
                denom: DEPOSIT_DENOM.load(deps.storage)?,
                amount: BALANCES
                    .may_load(deps.storage, &token_id)?
                    .unwrap_or_default(),
            }),
        },

        // Use default cw721-base query implementation
        _ => Cw721Contract::default().query(deps, env, msg),
    }
}
