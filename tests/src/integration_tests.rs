use soroban_sdk::Env;
use sphene_compliance::ComplianceClient;
use sphene_mock_usdc::MockUsdcClient;
use sphene_receipt_token::ReceiptTokenClient;
use sphene_vault::VaultClient;

use crate::setup::{allow, configure, deploy, mint_asset, PAR_NAV};

#[test]
fn deposit_blocked_transfer_allowlist_retry_and_withdraw() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    configure(&env, &contracts, PAR_NAV);
    allow(&env, &contracts, &contracts.alice);

    let deposit = 50_000_000_i128;
    let moved = 20_000_000_i128;
    mint_asset(&env, &contracts, &contracts.alice, deposit);

    let vault = VaultClient::new(&env, &contracts.vault);
    let asset = MockUsdcClient::new(&env, &contracts.asset);
    let receipt = ReceiptTokenClient::new(&env, &contracts.receipt);

    assert!(vault.try_deposit(&contracts.bob, &1).is_err());
    assert_eq!(vault.deposit(&contracts.alice, &deposit), deposit);
    assert_eq!(vault.total_assets(), deposit);
    assert_eq!(vault.total_shares(), deposit);
    assert_eq!(vault.asset_balance(), deposit);
    assert_eq!(asset.balance(&contracts.alice), 0);
    assert_eq!(receipt.balance(&contracts.alice), deposit);
    assert_eq!(
        asset.symbol(),
        soroban_sdk::String::from_str(&env, "mUSDC")
    );

    let alice_shares = receipt.balance(&contracts.alice);
    assert!(receipt
        .try_transfer(&contracts.alice, &contracts.bob, &moved)
        .is_err());
    assert_eq!(receipt.balance(&contracts.alice), alice_shares);
    assert_eq!(receipt.balance(&contracts.bob), 0);
    assert_eq!(vault.total_assets(), deposit);

    ComplianceClient::new(&env, &contracts.compliance).add_to_allowlist(&contracts.bob);
    receipt.transfer(&contracts.alice, &contracts.bob, &moved);
    assert_eq!(receipt.balance(&contracts.alice), deposit - moved);
    assert_eq!(receipt.balance(&contracts.bob), moved);
    assert_eq!(vault.shares(&contracts.bob), moved);

    ComplianceClient::new(&env, &contracts.compliance).remove_from_allowlist(&contracts.bob);
    assert!(vault.try_withdraw(&contracts.bob, &moved).is_err());
    assert_eq!(receipt.balance(&contracts.bob), moved);
    assert_eq!(asset.balance(&contracts.bob), 0);

    ComplianceClient::new(&env, &contracts.compliance).add_to_allowlist(&contracts.bob);
    assert_eq!(vault.withdraw(&contracts.bob, &moved), moved);
    assert_eq!(asset.balance(&contracts.bob), moved);
    assert_eq!(receipt.balance(&contracts.bob), 0);

    assert_eq!(vault.withdraw(&contracts.alice, &(deposit - moved)), deposit - moved);
    assert_eq!(asset.balance(&contracts.alice), deposit - moved);
    assert_eq!(vault.total_assets(), 0);
    assert_eq!(vault.total_shares(), 0);
    assert_eq!(vault.asset_balance(), 0);
    assert_eq!(receipt.total_supply(), 0);
}

#[test]
fn initialization_and_configuration_failures_leave_balances_unchanged() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    mint_asset(&env, &contracts, &contracts.alice, 10);
    let vault = VaultClient::new(&env, &contracts.vault);
    let asset = MockUsdcClient::new(&env, &contracts.asset);

    assert!(vault.try_deposit(&contracts.alice, &10).is_err());
    assert_eq!(asset.balance(&contracts.alice), 10);
    assert!(!vault.initialized());

    let receipt = ReceiptTokenClient::new(&env, &contracts.receipt);
    receipt.set_compliance(&contracts.compliance);
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &PAR_NAV)
        .is_err());
    receipt.set_minter(&contracts.alice);
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &PAR_NAV)
        .is_err());
    assert_eq!(asset.balance(&contracts.alice), 10);
    assert!(!vault.initialized());
}
