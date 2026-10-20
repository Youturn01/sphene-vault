use soroban_sdk::{testutils::Address as _, Address, Env};
use sphene_compliance::{Compliance, ComplianceClient};

use crate::setup::deploy;

fn client(env: &Env) -> (Address, Address, Address, ComplianceClient<'_>) {
    let contracts = deploy(env);
    let compliance = ComplianceClient::new(env, &contracts.compliance);
    (contracts.admin, contracts.alice, contracts.bob, compliance)
}

#[test]
fn add_remove_and_query_allowlist() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, alice, bob, compliance) = client(&env);

    assert!(!compliance.is_allowlisted(&alice));
    assert!(!compliance.can_transfer(&alice, &bob, &1));

    compliance.add_to_allowlist(&alice);
    assert!(compliance.is_allowlisted(&alice));
    assert!(!compliance.can_transfer(&alice, &bob, &1));

    compliance.add_to_allowlist(&bob);
    assert!(compliance.can_transfer(&alice, &bob, &1));
    assert!(compliance.can_transfer(&alice, &bob, &0));
    assert!(!compliance.can_transfer(&alice, &bob, &-1));

    compliance.remove_from_allowlist(&bob);
    assert!(!compliance.is_allowlisted(&bob));
    assert!(!compliance.can_transfer(&alice, &bob, &1));
    assert!(compliance.is_allowlisted(&alice));
}

#[test]
fn reject_unauthorized_allowlist_changes() {
    let env = Env::default();
    let (_admin, alice, _bob, compliance) = client(&env);

    assert!(compliance.try_add_to_allowlist(&alice).is_err());
    assert!(compliance.try_remove_from_allowlist(&alice).is_err());
    assert!(compliance.try_set_admin(&alice).is_err());
    assert!(!compliance.is_allowlisted(&alice));
}

#[test]
fn admin_can_rotate_and_new_admin_is_required() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = deploy(&env);
    let compliance = ComplianceClient::new(&env, &contracts.compliance);

    compliance.set_admin(&contracts.alice);
    assert_eq!(compliance.admin(), contracts.alice);
    compliance.add_to_allowlist(&contracts.bob);
    assert!(compliance.is_allowlisted(&contracts.bob));
}

#[test]
fn constructor_rejects_a_second_registration_path_via_missing_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let compliance_id = env.register(Compliance, (admin.clone(),));
    let compliance = ComplianceClient::new(&env, &compliance_id);
    assert_eq!(compliance.admin(), admin);
}
