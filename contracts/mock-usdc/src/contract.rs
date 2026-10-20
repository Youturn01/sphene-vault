//! SEP-41 mock asset. Administrative minting is unrestricted by an allowlist
//! because this contract is only a funding token for the vault prototype.
use crate::admin::{extend_instance, read_administrator, write_administrator};
use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
use soroban_sdk::{
    contract, contractevent, contractimpl, token::TokenInterface, Address, Env, MuxedAddress,
    String,
};
use soroban_token_sdk::events;
use soroban_token_sdk::metadata::TokenMetadata;

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount);
    }
}

#[contract]
pub struct MockUsdc;

#[contractevent(data_format = "single-value")]
pub struct SetAdmin {
    #[topic]
    admin: Address,
    new_admin: Address,
}

#[contractimpl]
impl MockUsdc {
    /// `name` and `symbol` are stored as given. Callers must pass
    /// "Mock USD Coin" and "mUSDC". This contract is not official USDC.
    pub fn __constructor(env: Env, admin: Address, decimal: u32, name: String, symbol: String) {
        if decimal > 18 {
            panic!("Decimal must not be greater than 18");
        }
        write_administrator(&env, &admin);
        write_metadata(
            &env,
            TokenMetadata {
                decimal,
                name,
                symbol,
            },
        );
        extend_instance(&env);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        read_administrator(&env).require_auth();
        extend_instance(&env);
        receive_balance(&env, to.clone(), amount);
        events::MintWithAmountOnly { to, amount }.publish(&env);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = read_administrator(&env);
        admin.require_auth();
        extend_instance(&env);
        write_administrator(&env, &new_admin);
        SetAdmin { admin, new_admin }.publish(&env);
    }
}

#[contractimpl(contracttrait)]
impl TokenInterface for MockUsdc {
    fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        extend_instance(&env);
        read_allowance(&env, from, spender).amount
    }

    fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        from.require_auth();
        check_nonnegative_amount(amount);
        extend_instance(&env);
        write_allowance(&env, from.clone(), spender.clone(), amount, live_until_ledger);
        events::Approve {
            from,
            spender,
            amount,
            live_until_ledger,
        }
        .publish(&env);
    }

    fn balance(env: Env, id: Address) -> i128 {
        extend_instance(&env);
        read_balance(&env, id)
    }

    fn transfer(env: Env, from: Address, to_muxed: MuxedAddress, amount: i128) {
        from.require_auth();
        check_nonnegative_amount(amount);
        extend_instance(&env);
        spend_balance(&env, from.clone(), amount);
        let to = to_muxed.address();
        receive_balance(&env, to.clone(), amount);
        events::Transfer {
            from,
            to,
            to_muxed_id: to_muxed.id(),
            amount,
        }
        .publish(&env);
    }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        check_nonnegative_amount(amount);
        extend_instance(&env);
        spend_allowance(&env, from.clone(), spender, amount);
        spend_balance(&env, from.clone(), amount);
        receive_balance(&env, to.clone(), amount);
        events::Transfer {
            from,
            to,
            to_muxed_id: None,
            amount,
        }
        .publish(&env);
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        check_nonnegative_amount(amount);
        extend_instance(&env);
        spend_balance(&env, from.clone(), amount);
        events::Burn { from, amount }.publish(&env);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        check_nonnegative_amount(amount);
        extend_instance(&env);
        spend_allowance(&env, from.clone(), spender, amount);
        spend_balance(&env, from.clone(), amount);
        events::Burn { from, amount }.publish(&env);
    }

    fn decimals(env: Env) -> u32 {
        read_decimal(&env)
    }

    fn name(env: Env) -> String {
        read_name(&env)
    }

    fn symbol(env: Env) -> String {
        read_symbol(&env)
    }
}
