use soroban_sdk::{Address, Env};

use crate::storage_types::{AllowanceDataKey, AllowanceValue, DataKey};

pub fn read_allowance(env: &Env, from: Address, spender: Address) -> AllowanceValue {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    if let Some(allowance) = env
        .storage()
        .temporary()
        .get::<_, AllowanceValue>(&key)
    {
        if allowance.live_until_ledger < env.ledger().sequence() {
            AllowanceValue {
                amount: 0,
                live_until_ledger: allowance.live_until_ledger,
            }
        } else {
            allowance
        }
    } else {
        AllowanceValue {
            amount: 0,
            live_until_ledger: 0,
        }
    }
}

pub fn write_allowance(
    env: &Env,
    from: Address,
    spender: Address,
    amount: i128,
    live_until_ledger: u32,
) {
    if amount > 0 && live_until_ledger < env.ledger().sequence() {
        panic!("live_until_ledger is less than ledger seq when amount > 0");
    }

    let allowance = AllowanceValue {
        amount,
        live_until_ledger,
    };
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    env.storage().temporary().set(&key, &allowance);
    if amount > 0 {
        let live_for = live_until_ledger
            .checked_sub(env.ledger().sequence())
            .expect("allowance lifetime underflow");
        env.storage()
            .temporary()
            .extend_ttl(&key, live_for, live_for);
    }
}

pub fn spend_allowance(env: &Env, from: Address, spender: Address, amount: i128) {
    let allowance = read_allowance(env, from.clone(), spender.clone());
    if allowance.amount < amount {
        panic!("insufficient allowance");
    }
    if amount > 0 {
        write_allowance(
            env,
            from,
            spender,
            allowance
                .amount
                .checked_sub(amount)
                .expect("allowance underflow"),
            allowance.live_until_ledger,
        );
    }
}
