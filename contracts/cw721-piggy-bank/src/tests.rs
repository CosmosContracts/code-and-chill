use crate::{
    contract::{execute, instantiate, CONTRACT_NAME},
    msg::{ExecuteExt, ExecuteMsg, InstantiateMsg, MetadataExt},
    ContractError,
};

use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, CosmosMsg, StdError,
};

/// Make sure cw2 version info is properly initialized during instantiation,
/// and NOT overwritten by the base contract.
#[test]
fn proper_cw2_initialization() {
    let mut deps = mock_dependencies();

    instantiate(
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

    instantiate(
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
    execute(
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
    execute(
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
    execute(
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
    let err = execute(
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
            kind: "cw721_base::state::TokenInfo<cw721_piggy_bank::msg::MetadataExt>".to_string()
        })
    );

    // Calling deposit succeeds with correct token
    execute(
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
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("rando", &[]),
        ExecuteMsg::Burn {
            token_id: "1".to_string(),
        },
    )
    .unwrap_err();

    // Can't burn an NFT that doesn't exist
    let err = execute(
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
            kind: "cw721_base::state::TokenInfo<cw721_piggy_bank::msg::MetadataExt>".to_string()
        }))
    );

    // Test burning NFT returns money
    let res = execute(
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
