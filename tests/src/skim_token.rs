//! Test double whose balance can be reduced without the vault's consent.
//! It is not deployed and it is not a stand-in for USDC.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, MuxedAddress};

#[derive(Clone)]
#[contracttype]
enum Key {
    Balance(Address),
}

#[contract]
pub struct SkimToken;

#[contractimpl]
impl SkimToken {
    pub fn decimals(_env: Env) -> u32 {
        7
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let updated = read_balance(&env, to.clone())
            .checked_add(amount)
            .expect("balance overflow");
        write_balance(&env, to, updated);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, id)
    }

    pub fn skim(env: Env, from: Address, amount: i128) {
        let balance = read_balance(&env, from.clone());
        if balance < amount {
            panic!("insufficient balance");
        }
        write_balance(
            &env,
            from,
            balance.checked_sub(amount).expect("balance underflow"),
        );
    }

    pub fn transfer(env: Env, from: Address, to_muxed: MuxedAddress, amount: i128) {
        from.require_auth();
        if amount < 0 {
            panic!("negative amount");
        }
        let balance = read_balance(&env, from.clone());
        if balance < amount {
            panic!("insufficient balance");
        }
        write_balance(
            &env,
            from,
            balance.checked_sub(amount).expect("balance underflow"),
        );
        let to = to_muxed.address();
        let updated = read_balance(&env, to.clone())
            .checked_add(amount)
            .expect("balance overflow");
        write_balance(&env, to, updated);
    }
}

fn read_balance(env: &Env, id: Address) -> i128 {
    env.storage()
        .persistent()
        .get(&Key::Balance(id))
        .unwrap_or(0)
}

fn write_balance(env: &Env, id: Address, amount: i128) {
    env.storage().persistent().set(&Key::Balance(id), &amount);
}
