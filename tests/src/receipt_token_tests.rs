use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Env, IntoVal, String, Symbol, Val,
};
use sphene_compliance::ComplianceClient;
use sphene_receipt_token::{ReceiptToken, ReceiptTokenClient};

use crate::setup::{allow, configure, deploy, PAR_NAV};

const LIVE: u32 = 1_000;

fn events_with_topic(env: &Env, contract: &soroban_sdk::Address, topic: &str) -> usize {
    let expected: Val = Symbol::new(env, topic).into_val(env);
    env.events()
        .all()
        .iter()
        .filter(|(id, topics, _data)| id == contract && topics.get(0) == Some(expected))
        .count()
}

#[test]
fn metadata_balances_allowances_and_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    configure(&env, &contracts, PAR_NAV);
    allow(&env, &contracts, &contracts.alice);
    allow(&env, &contracts, &contracts.bob);

    let token = ReceiptTokenClient::new(&env, &contracts.receipt);
    assert_eq!(token.decimals(), 7);
    assert_eq!(
        token.name(),
        String::from_str(&env, "Sphene Receipt")
    );
    assert_eq!(token.symbol(), String::from_str(&env, "sphRWA"));
    assert_eq!(
        sphene_mock_usdc::MockUsdcClient::new(&env, &contracts.asset).symbol(),
        String::from_str(&env, "mUSDC")
    );

    assert_eq!(token.mint(&contracts.alice, &1_000), ());
    assert_eq!(token.balance(&contracts.alice), 1_000);
    assert_eq!(token.total_supply(), 1_000);
    assert_eq!(
        events_with_topic(&env, &contracts.receipt, "mint"),
        1
    );

    token.approve(&contracts.alice, &contracts.carol, &400, &LIVE);
    assert_eq!(token.allowance(&contracts.alice, &contracts.carol), 400);
    assert_eq!(
        events_with_topic(&env, &contracts.receipt, "approve"),
        1
    );

    token.transfer(&contracts.alice, &contracts.bob, &250);
    assert_eq!(token.balance(&contracts.alice), 750);
    assert_eq!(token.balance(&contracts.bob), 250);
    assert_eq!(token.total_supply(), 1_000);
    assert_eq!(
        events_with_topic(&env, &contracts.receipt, "transfer"),
        1
    );

    token.transfer_from(&contracts.carol, &contracts.alice, &contracts.bob, &150);
    assert_eq!(token.balance(&contracts.alice), 600);
    assert_eq!(token.balance(&contracts.bob), 400);
    assert_eq!(token.allowance(&contracts.alice, &contracts.carol), 250);

    token.burn_from(&contracts.carol, &contracts.alice, &50);
    assert_eq!(token.balance(&contracts.alice), 550);
    assert_eq!(token.allowance(&contracts.alice, &contracts.carol), 200);
    assert_eq!(token.total_supply(), 950);
    assert_eq!(events_with_topic(&env, &contracts.receipt, "burn"), 1);

    token.burn(&contracts.alice, &50);
    assert_eq!(token.balance(&contracts.alice), 500);
    assert_eq!(token.total_supply(), 900);
}

#[test]
fn allowance_expiry_and_zero_amount_transfer_from() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    configure(&env, &contracts, PAR_NAV);
    allow(&env, &contracts, &contracts.alice);
    allow(&env, &contracts, &contracts.bob);
    let token = ReceiptTokenClient::new(&env, &contracts.receipt);
    token.mint(&contracts.alice, &10);
    env.ledger().set_sequence_number(1);

    assert!(token
        .try_approve(&contracts.alice, &contracts.bob, &5, &0)
        .is_err());

    token.approve(&contracts.alice, &contracts.bob, &5, &10);
    assert_eq!(token.allowance(&contracts.alice, &contracts.bob), 5);
    env.ledger().set_sequence_number(11);
    assert_eq!(token.allowance(&contracts.alice, &contracts.bob), 0);
    assert!(token
        .try_transfer_from(&contracts.bob, &contracts.alice, &contracts.bob, &1)
        .is_err());
    assert_eq!(token.balance(&contracts.alice), 10);

    token.approve(&contracts.alice, &contracts.bob, &0, &0);
    assert!(token
        .try_transfer_from(&contracts.bob, &contracts.alice, &contracts.bob, &0)
        .is_ok());
    assert_eq!(token.balance(&contracts.alice), 10);
    assert_eq!(token.balance(&contracts.bob), 0);
}

#[test]
fn reject_unauthorized_mint_and_burn() {
    let env = Env::default();
    let contracts = deploy(&env);
    let token = ReceiptTokenClient::new(&env, &contracts.receipt);
    assert!(token.try_mint(&contracts.alice, &1).is_err());
    assert!(token.try_vault_burn(&contracts.alice, &1).is_err());
    assert!(token.try_set_admin(&contracts.alice).is_err());
    assert!(token.try_set_minter(&contracts.vault).is_err());
    assert!(token.try_set_compliance(&contracts.compliance).is_err());
    assert_eq!(token.total_supply(), 0);
    assert_eq!(token.balance(&contracts.alice), 0);
}

#[test]
fn reject_direct_and_delegated_transfers_that_fail_the_allowlist() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    configure(&env, &contracts, PAR_NAV);
    allow(&env, &contracts, &contracts.alice);
    let token = ReceiptTokenClient::new(&env, &contracts.receipt);
    token.mint(&contracts.alice, &80);
    token.approve(&contracts.alice, &contracts.carol, &80, &LIVE);

    let supply = token.total_supply();
    let alice_before = token.balance(&contracts.alice);
    let allowance_before = token.allowance(&contracts.alice, &contracts.carol);
    let events_before = env.events().all().len();

    assert!(token
        .try_transfer(&contracts.alice, &contracts.bob, &30)
        .is_err());
    assert!(token
        .try_transfer_from(&contracts.carol, &contracts.alice, &contracts.bob, &30)
        .is_err());
    assert!(token.try_transfer(&contracts.alice, &contracts.bob, &0).is_err());
    assert!(token.try_transfer(&contracts.alice, &contracts.bob, &-5).is_err());
    assert!(token.try_burn(&contracts.bob, &1).is_err());

    assert_eq!(token.balance(&contracts.alice), alice_before);
    assert_eq!(token.balance(&contracts.bob), 0);
    assert_eq!(token.allowance(&contracts.alice, &contracts.carol), allowance_before);
    assert_eq!(token.total_supply(), supply);
    assert_eq!(env.events().all().len(), events_before);

    allow(&env, &contracts, &contracts.bob);
    token.transfer(&contracts.alice, &contracts.bob, &30);
    token.transfer_from(&contracts.carol, &contracts.alice, &contracts.bob, &20);
    assert_eq!(token.balance(&contracts.alice), 30);
    assert_eq!(token.balance(&contracts.bob), 50);
    assert_eq!(token.allowance(&contracts.alice, &contracts.carol), 60);

    token.transfer(&contracts.alice, &contracts.alice, &0);
    assert_eq!(token.balance(&contracts.alice), 30);
}

#[test]
fn mint_to_an_unlisted_address_fails_and_delisting_blocks_burn() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    configure(&env, &contracts, PAR_NAV);
    let token = ReceiptTokenClient::new(&env, &contracts.receipt);

    assert!(token.try_mint(&contracts.alice, &5).is_err());
    assert_eq!(token.total_supply(), 0);

    allow(&env, &contracts, &contracts.alice);
    token.mint(&contracts.alice, &5);
    ComplianceClient::new(&env, &contracts.compliance).remove_from_allowlist(&contracts.alice);
    assert!(token.try_burn(&contracts.alice, &5).is_err());
    assert!(token.try_transfer(&contracts.alice, &contracts.alice, &1).is_err());
    assert_eq!(token.balance(&contracts.alice), 5);
    assert_eq!(token.total_supply(), 5);
}

#[test]
fn configuration_cannot_be_repeated() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    let token = ReceiptTokenClient::new(&env, &contracts.receipt);
    token.set_compliance(&contracts.compliance);
    token.set_minter(&contracts.vault);
    assert!(token.try_set_compliance(&contracts.compliance).is_err());
    assert!(token.try_set_minter(&contracts.alice).is_err());
    assert_eq!(token.compliance(), contracts.compliance);
    assert_eq!(token.minter(), contracts.vault);
}

#[test]
#[should_panic(expected = "Decimal must not be greater than 18")]
fn decimal_above_eighteen_is_rejected() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let _id = env.register(
        ReceiptToken,
        (
            admin,
            19_u32,
            String::from_str(&env, "nope"),
            String::from_str(&env, "NO"),
        ),
    );
}
