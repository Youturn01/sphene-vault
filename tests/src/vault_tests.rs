use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction},
    Address, Env,
};
use sphene_compliance::ComplianceClient;
use sphene_mock_usdc::MockUsdcClient;
use sphene_receipt_token::ReceiptTokenClient;
use sphene_vault::VaultClient;

use crate::setup::{allow, configure, deploy, mint_asset, Contracts, PAR_NAV};
use crate::skim_token::SkimTokenClient;

fn ready(env: &Env, nav: i128) -> Contracts {
    env.mock_all_auths();
    let contracts = deploy(env);
    configure(env, &contracts, nav);
    allow(env, &contracts, &contracts.alice);
    allow(env, &contracts, &contracts.bob);
    contracts
}

#[test]
fn deposit_and_withdraw_at_par_nav() {
    let env = Env::default();
    let contracts = ready(&env, PAR_NAV);
    mint_asset(&env, &contracts, &contracts.alice, 50_000_000);

    let vault = VaultClient::new(&env, &contracts.vault);
    let asset = MockUsdcClient::new(&env, &contracts.asset);
    let receipt = ReceiptTokenClient::new(&env, &contracts.receipt);

    assert_eq!(vault.preview_deposit(&50_000_000), 50_000_000);
    let shares = vault.deposit(&contracts.alice, &50_000_000);
    assert_eq!(shares, 50_000_000);
    assert_eq!(vault.total_assets(), 50_000_000);
    assert_eq!(vault.total_shares(), 50_000_000);
    assert_eq!(vault.shares(&contracts.alice), 50_000_000);
    assert_eq!(vault.asset_balance(), 50_000_000);
    assert_eq!(asset.balance(&contracts.alice), 0);
    assert_eq!(receipt.balance(&contracts.alice), 50_000_000);
    assert_eq!(vault.nav(), PAR_NAV);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, contracts.alice);
    match &auths[0].1.function {
        AuthorizedFunction::Contract((id, name, _)) => {
            assert_eq!(id, &contracts.vault);
            assert_eq!(name, &symbol_short!("deposit"));
        }
        other => panic!("unexpected auth: {:?}", other),
    }
    assert!(!auths[0].1.sub_invocations.is_empty());

    let assets = vault.withdraw(&contracts.alice, &20_000_000);
    assert_eq!(assets, 20_000_000);
    assert_eq!(vault.shares(&contracts.alice), 30_000_000);
    assert_eq!(vault.total_assets(), 30_000_000);
    assert_eq!(vault.total_shares(), 30_000_000);
    assert_eq!(asset.balance(&contracts.alice), 20_000_000);
    assert_eq!(vault.asset_balance(), 30_000_000);

    let rest = vault.withdraw(&contracts.alice, &30_000_000);
    assert_eq!(rest, 30_000_000);
    assert_eq!(vault.total_assets(), 0);
    assert_eq!(vault.total_shares(), 0);
    assert_eq!(vault.asset_balance(), 0);
    assert_eq!(asset.balance(&contracts.alice), 50_000_000);
    assert_eq!(receipt.total_supply(), 0);
}

#[test]
fn two_depositors_keep_separate_shares() {
    let env = Env::default();
    let contracts = ready(&env, PAR_NAV);
    mint_asset(&env, &contracts, &contracts.alice, 10_000_000);
    mint_asset(&env, &contracts, &contracts.bob, 25_000_000);
    let vault = VaultClient::new(&env, &contracts.vault);

    vault.deposit(&contracts.alice, &10_000_000);
    vault.deposit(&contracts.bob, &25_000_000);

    assert_eq!(vault.shares(&contracts.alice), 10_000_000);
    assert_eq!(vault.shares(&contracts.bob), 25_000_000);
    assert_eq!(vault.total_assets(), 35_000_000);
    assert_eq!(vault.total_shares(), 35_000_000);

    vault.withdraw(&contracts.bob, &25_000_000);
    assert_eq!(vault.shares(&contracts.alice), 10_000_000);
    assert_eq!(vault.total_assets(), 10_000_000);
    assert_eq!(
        MockUsdcClient::new(&env, &contracts.asset).balance(&contracts.alice),
        0
    );
}

#[test]
fn fixed_nav_above_par_and_rounding_down() {
    let env = Env::default();
    let contracts = ready(&env, 20_000_000);
    mint_asset(&env, &contracts, &contracts.alice, 20_000_000);
    let vault = VaultClient::new(&env, &contracts.vault);

    assert_eq!(vault.preview_deposit(&20_000_000), 10_000_000);
    assert_eq!(vault.deposit(&contracts.alice, &20_000_000), 10_000_000);
    assert_eq!(vault.preview_withdraw(&10_000_000), 20_000_000);
    assert_eq!(vault.withdraw(&contracts.alice, &10_000_000), 20_000_000);
    assert_eq!(vault.total_assets(), 0);

    let rounding = ready(&env, 3);
    mint_asset(&env, &rounding, &rounding.alice, 10);
    let rounding_vault = VaultClient::new(&env, &rounding.vault);
    assert_eq!(rounding_vault.preview_deposit(&10), 33_333_333);
    assert_eq!(rounding_vault.deposit(&rounding.alice, &10), 33_333_333);
    assert_eq!(rounding_vault.preview_withdraw(&33_333_333), 9);
    assert_eq!(rounding_vault.withdraw(&rounding.alice, &33_333_333), 9);
    assert_eq!(rounding_vault.total_assets(), 1);
    assert_eq!(rounding_vault.total_shares(), 0);
    assert_eq!(rounding_vault.asset_balance(), 1);
    assert_eq!(
        MockUsdcClient::new(&env, &rounding.asset).balance(&rounding.alice),
        9
    );
}

#[test]
fn reject_zero_negative_and_overflow_deposits() {
    let env = Env::default();
    let contracts = ready(&env, PAR_NAV);
    mint_asset(&env, &contracts, &contracts.alice, 5);
    let vault = VaultClient::new(&env, &contracts.vault);
    let asset = MockUsdcClient::new(&env, &contracts.asset);

    assert!(vault.try_deposit(&contracts.alice, &0).is_err());
    assert!(vault.try_deposit(&contracts.alice, &-1).is_err());
    assert!(vault.try_deposit(&contracts.alice, &i128::MAX).is_err());
    assert!(vault.try_preview_deposit(&-5).is_err());
    assert_eq!(asset.balance(&contracts.alice), 5);
    assert_eq!(vault.total_assets(), 0);
    assert_eq!(vault.total_shares(), 0);

    let high = ready(&env, 20_000_000);
    mint_asset(&env, &high, &high.alice, 1);
    let high_vault = VaultClient::new(&env, &high.vault);
    assert_eq!(high_vault.preview_deposit(&1), 0);
    assert!(high_vault.try_deposit(&high.alice, &1).is_err());
    assert_eq!(
        MockUsdcClient::new(&env, &high.asset).balance(&high.alice),
        1
    );
}

#[test]
fn reject_insufficient_shares_and_zero_withdrawal() {
    let env = Env::default();
    let contracts = ready(&env, PAR_NAV);
    mint_asset(&env, &contracts, &contracts.alice, 15);
    let vault = VaultClient::new(&env, &contracts.vault);
    vault.deposit(&contracts.alice, &15);

    assert!(vault.try_withdraw(&contracts.alice, &16).is_err());
    assert!(vault.try_withdraw(&contracts.alice, &0).is_err());
    assert!(vault.try_withdraw(&contracts.alice, &-1).is_err());
    assert!(vault.try_withdraw(&contracts.bob, &1).is_err());
    assert_eq!(vault.shares(&contracts.alice), 15);
    assert_eq!(vault.total_assets(), 15);
    assert_eq!(vault.asset_balance(), 15);
}

#[test]
fn reject_withdrawal_when_the_token_balance_was_reduced() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let asset = env.register(SkimToken, ());
    let compliance = env.register(
        sphene_compliance::Compliance,
        (admin.clone(),),
    );
    let receipt = env.register(
        sphene_receipt_token::ReceiptToken,
        (
            admin.clone(),
            7_u32,
            soroban_sdk::String::from_str(&env, "Sphene Receipt"),
            soroban_sdk::String::from_str(&env, "sphRWA"),
        ),
    );
    let vault_id = env.register(sphene_vault::Vault, (admin.clone(),));

    let receipt_client = ReceiptTokenClient::new(&env, &receipt);
    receipt_client.set_compliance(&compliance);
    receipt_client.set_minter(&vault_id);
    ComplianceClient::new(&env, &compliance).add_to_allowlist(&alice);

    let vault = VaultClient::new(&env, &vault_id);
    vault.initialize(&asset, &receipt, &PAR_NAV);
    SkimTokenClient::new(&env, &asset).mint(&alice, &100);
    assert_eq!(vault.deposit(&alice, &100), 100);

    SkimTokenClient::new(&env, &asset).skim(&vault_id, &40);
    assert_eq!(vault.total_assets(), 100);
    assert_eq!(vault.asset_balance(), 60);
    assert!(vault.try_withdraw(&alice, &100).is_err());
    assert_eq!(vault.shares(&alice), 100);
    assert_eq!(vault.total_assets(), 100);

    assert_eq!(vault.withdraw(&alice, &60), 60);
    assert_eq!(vault.shares(&alice), 40);
    assert_eq!(vault.asset_balance(), 0);
    assert!(vault.try_withdraw(&alice, &40).is_err());
    assert_eq!(receipt_client.balance(&alice), 40);
}

#[test]
fn reject_bad_configuration_and_unauthorized_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    let vault = VaultClient::new(&env, &contracts.vault);
    let receipt = ReceiptTokenClient::new(&env, &contracts.receipt);

    assert!(!vault.initialized());
    assert!(vault.try_deposit(&contracts.alice, &1).is_err());
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &PAR_NAV)
        .is_err());

    receipt.set_minter(&contracts.vault);
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &PAR_NAV)
        .is_err());

    receipt.set_compliance(&contracts.compliance);
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &0)
        .is_err());
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &-10)
        .is_err());
    assert!(vault
        .try_initialize(&contracts.receipt, &contracts.receipt, &PAR_NAV)
        .is_err());
    assert!(vault
        .try_initialize(&contracts.compliance, &contracts.receipt, &PAR_NAV)
        .is_err());

    vault.initialize(&contracts.asset, &contracts.receipt, &PAR_NAV);
    assert!(vault.initialized());
    assert!(vault
        .try_initialize(&contracts.asset, &contracts.receipt, &PAR_NAV)
        .is_err());
    assert_eq!(vault.asset(), contracts.asset);
    assert_eq!(vault.receipt(), contracts.receipt);
    assert_eq!(vault.scale(), PAR_NAV);

    let stranger = Address::generate(&env);
    vault.set_admin(&stranger);
    assert_eq!(vault.admin(), stranger);

    let locked = Env::default();
    let deployed = deploy(&locked);
    let locked_vault = VaultClient::new(&locked, &deployed.vault);
    assert!(locked_vault.try_set_admin(&deployed.alice).is_err());
    assert!(locked_vault
        .try_initialize(&deployed.asset, &deployed.receipt, &PAR_NAV)
        .is_err());
    assert_eq!(locked_vault.admin(), deployed.admin);
}

#[test]
fn deposit_without_a_balance_does_not_change_accounting() {
    let env = Env::default();
    let contracts = ready(&env, PAR_NAV);
    let vault = VaultClient::new(&env, &contracts.vault);
    assert!(vault.try_deposit(&contracts.alice, &10).is_err());
    assert_eq!(vault.total_assets(), 0);
    assert_eq!(vault.total_shares(), 0);
    assert_eq!(
        ReceiptTokenClient::new(&env, &contracts.receipt).balance(&contracts.alice),
        0
    );
}
