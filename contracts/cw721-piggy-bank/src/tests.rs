use crate::{
    contract::{execute, instantiate, query, CONTRACT_NAME},
    msg::{ExecuteExt, ExecuteMsg, InstantiateMsg, MetadataExt, QueryMsg},
    ContractError,
};

use cosmwasm_std::{
    coin, coins, from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, CosmosMsg, StdError,
};
use cw721::{AllNftInfoResponse, TokensResponse};

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
            mint_price: coin(1000000, "ujuno"),
            base_url: "https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link".to_string(),
            max_nft_supply: Some(2),
            sale_funds_recipient: "larry".into(),
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
            mint_price: coin(1000000, "ujuno"),
            base_url: "https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link".to_string(),
            max_nft_supply: Some(2),
            sale_funds_recipient: "larry".into(),
        },
    )
    .unwrap();

    // Mint the NFT fails when no funds are sent
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(BOB, &[]),
        ExecuteMsg::Mint {
            token_id: "1".into(),
            owner: BOB.into(),
            token_uri: Some("https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link/1/seedling.json".to_string()),
            extension: MetadataExt {},
        },
    )
    .unwrap_err();

    // Mint the NFT succeeds if correct amount of funds are sent
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(BOB, &coins(1000000, "ujuno")),
        ExecuteMsg::Mint {
            // These fields are ignored, we don't need them
            // TODO fix types for this message, these are all ignored
            token_id: "1".into(),
            owner: BOB.into(),
            token_uri: Some("https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link/1/seedling.json".to_string()),
            extension: MetadataExt {},
        },
    )
    .unwrap();
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "larry".to_string(),
            amount: coins(1000000, "ujuno"),
        })
    );

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

    let token: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", token);

    // Check token URI is seedling state
    let token: AllNftInfoResponse<MetadataExt> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllNftInfo {
                token_id: "1".to_string(),
                include_expired: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(token.info.token_uri, Some("https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link/1/seedling.json".to_string()));

    // Deposit enough to change the token URI
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(BOB, &coins(1000000, "ujuno")),
        ExecuteMsg::Extension {
            msg: ExecuteExt::Deposit {
                token_id: "1".to_string(),
            },
        },
    )
    .unwrap();

    // Check token URI is sapling state
    let token: AllNftInfoResponse<MetadataExt> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllNftInfo {
                token_id: "1".to_string(),
                include_expired: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(token.info.token_uri, Some("https://bafybeie2grcflzjvds7i33bxjjgktjdfcp2h2v27gdkbyuiaelvbgtdewy.ipfs.nftstorage.link/1/seedling.json".to_string()));

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
            amount: coins(1000000, "ujuno"),
        })
    );
}
