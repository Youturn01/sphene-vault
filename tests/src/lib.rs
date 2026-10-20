//! Automated tests for Sphene Vault. Run them with `cargo test --workspace`.

#[cfg(test)]
mod setup;
#[cfg(test)]
mod skim_token;
#[cfg(test)]
mod compliance_tests;
#[cfg(test)]
mod vault_tests;
#[cfg(test)]
mod receipt_token_tests;
#[cfg(test)]
mod integration_tests;
