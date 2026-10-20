#![no_std]

//! Allowlist consulted by the Sphene receipt token.
//!
//! `can_transfer` permits a movement only when the amount is non-negative and
//! both the sender and the recipient are allowlisted. The receipt token treats
//! a `false` result as a hard failure and reverts.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;
const ENTRY_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const ENTRY_LIFETIME_THRESHOLD: u32 = ENTRY_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    Allowlisted(Address),
}

#[contract]
pub struct Compliance;

#[contractimpl]
impl Compliance {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        extend_instance(&env);
    }

    pub fn add_to_allowlist(env: Env, address: Address) {
        read_admin(&env).require_auth();
        extend_instance(&env);
        write_allowlisted(&env, &address, true);
    }

    pub fn remove_from_allowlist(env: Env, address: Address) {
        read_admin(&env).require_auth();
        extend_instance(&env);
        write_allowlisted(&env, &address, false);
    }

    pub fn is_allowlisted(env: Env, address: Address) -> bool {
        extend_instance(&env);
        allowlisted(&env, &address)
    }

    /// Both parties must be allowlisted. A negative amount is never permitted.
    /// Zero is permitted only between allowlisted addresses; the token standard
    /// used by this project allows a zero-amount transfer.
    pub fn can_transfer(env: Env, from: Address, to: Address, amount: i128) -> bool {
        extend_instance(&env);
        if amount < 0 {
            return false;
        }
        allowlisted(&env, &from) && allowlisted(&env, &to)
    }

    pub fn admin(env: Env) -> Address {
        extend_instance(&env);
        read_admin(&env)
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = read_admin(&env);
        admin.require_auth();
        extend_instance(&env);
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }
}

fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("compliance is not initialized")
}

fn allowlisted(env: &Env, address: &Address) -> bool {
    let key = DataKey::Allowlisted(address.clone());
    if !env.storage().persistent().has(&key) {
        return false;
    }
    env.storage().persistent().extend_ttl(
        &key,
        ENTRY_LIFETIME_THRESHOLD,
        ENTRY_BUMP_AMOUNT,
    );
    env.storage().persistent().get(&key).unwrap_or(false)
}

fn write_allowlisted(env: &Env, address: &Address, allowed: bool) {
    let key = DataKey::Allowlisted(address.clone());
    env.storage().persistent().set(&key, &allowed);
    env.storage().persistent().extend_ttl(
        &key,
        ENTRY_LIFETIME_THRESHOLD,
        ENTRY_BUMP_AMOUNT,
    );
}

fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}
