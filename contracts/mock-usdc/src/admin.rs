use soroban_sdk::{Address, Env};

use crate::storage_types::DataKey;

pub fn read_administrator(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("token is not initialized")
}

pub fn write_administrator(env: &Env, id: &Address) {
    env.storage().instance().set(&DataKey::Admin, id);
}

pub fn extend_instance(env: &Env) {
    use crate::storage_types::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD};
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}
