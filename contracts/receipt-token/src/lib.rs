#![no_std]

//! SEP-41 receipt token for Sphene Vault.
//!
//! Every `transfer` and `transfer_from` asks the compliance contract before
//! balances change. Minting requires the recipient to be allowlisted. Burning
//! requires the holder to be allowlisted. The vault is the only minter, and
//! `vault_burn` is the only burn path that does not ask the holder to
//! authorize the token call directly: the holder authorizes `vault.withdraw`,
//! and the vault authorizes this burn.

mod admin;
mod allowance;
mod balance;
mod contract;
mod metadata;
mod storage_types;

pub use contract::ReceiptToken;
pub use contract::ReceiptTokenClient;
