#![no_std]

//! Mock USDC for the Sphene Vault prototype.
//!
//! This is a test token. It is not Circle USDC and it does not represent a
//! dollar deposit. Deployments must use the name "Mock USD Coin" and the
//! symbol "mUSDC".

mod admin;
mod allowance;
mod balance;
mod contract;
mod metadata;
mod storage_types;

pub use contract::MockUsdc;
pub use contract::MockUsdcClient;
