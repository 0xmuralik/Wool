use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Funds Mismatch")]
    FundsMismatched {
        // expected: String,
    // found: String,
    },

    #[error("Insufficient Funds")]
    InsufficientFunds {
        // expected: String,
    // found: String,
    },

    #[error("Pool id already in use")]
    AlreadyInUse {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
