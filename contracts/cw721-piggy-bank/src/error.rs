use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

/// Custom errors for this contract
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    /// This inherits from cw721-base::ContractError to handle the base contract errors
    #[error("NFT contract error: {0}")]
    Cw721Error(#[from] cw721_base::ContractError),
}
