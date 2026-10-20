use soroban_sdk::{Address, Env};

use crate::storage_types::{DataKey, INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD};

pub fn read_administrator(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("token is not initialized")
}

pub fn write_administrator(env: &Env, id: &Address) {
    env.storage().instance().set(&DataKey::Admin, id);
}

pub fn read_minter(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Minter)
        .expect("minter is not set")
}

pub fn read_compliance(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Compliance)
        .expect("compliance is not set")
}

pub fn read_supply(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::Supply).unwrap_or(0)
}

pub fn add_supply(env: &Env, amount: i128) {
    let updated = read_supply(env)
        .checked_add(amount)
        .expect("supply overflow");
    env.storage().instance().set(&DataKey::Supply, &updated);
}

pub fn sub_supply(env: &Env, amount: i128) {
    let supply = read_supply(env);
    if supply < amount {
        panic!("insufficient supply");
    }
    env.storage().instance().set(
        &DataKey::Supply,
        &supply.checked_sub(amount).expect("supply underflow"),
    );
}

pub fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}
