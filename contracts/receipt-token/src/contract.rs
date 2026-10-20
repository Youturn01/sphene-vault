//! SEP-41 receipt token. Compliance is enforced inside this contract.
use crate::admin::{
    add_supply, extend_instance, read_administrator, read_compliance, read_minter, read_supply,
    sub_supply, write_administrator,
};
use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
use crate::storage_types::DataKey;
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, token::TokenInterface, Address, Env,
    MuxedAddress, String,
};
use soroban_token_sdk::events;
use soroban_token_sdk::metadata::TokenMetadata;

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount);
    }
}

/// Local client so this contract does not link the compliance implementation
/// into its Wasm. The function names and arguments match `sphene_compliance`.
#[contractclient(name = "ComplianceClient")]
trait ComplianceInterface {
    fn can_transfer(env: Env, from: Address, to: Address, amount: i128) -> bool;
    fn is_allowlisted(env: Env, address: Address) -> bool;
}

fn require_transfer_allowed(env: &Env, from: &Address, to: &Address, amount: i128) {
    let compliance = read_compliance(env);
    let client = ComplianceClient::new(env, &compliance);
    if !client.can_transfer(from, to, &amount) {
        panic!("compliance rejected transfer");
    }
}

fn require_recipient_allowlisted(env: &Env, to: &Address) {
    let compliance = read_compliance(env);
    let client = ComplianceClient::new(env, &compliance);
    if !client.is_allowlisted(to) {
        panic!("compliance rejected transfer");
    }
}

#[contract]
pub struct ReceiptToken;

#[contractevent(data_format = "single-value")]
pub struct SetAdmin {
    #[topic]
    admin: Address,
    new_admin: Address,
}

#[contractimpl]
impl ReceiptToken {
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

    /// One-time configuration. The admin cannot retarget compliance after it
    /// is set, so a later admin change cannot swap in a permissive policy.
    pub fn set_compliance(env: Env, compliance: Address) {
        read_administrator(&env).require_auth();
        if env.storage().instance().has(&DataKey::Compliance) {
            panic!("compliance already set");
        }
        extend_instance(&env);
        env.storage()
            .instance()
            .set(&DataKey::Compliance, &compliance);
    }

    /// One-time configuration. Only this address may mint or call `vault_burn`.
    pub fn set_minter(env: Env, minter: Address) {
        read_administrator(&env).require_auth();
        if env.storage().instance().has(&DataKey::Minter) {
            panic!("minter already set");
        }
        extend_instance(&env);
        env.storage().instance().set(&DataKey::Minter, &minter);
    }

    pub fn compliance(env: Env) -> Address {
        extend_instance(&env);
        read_compliance(&env)
    }

    pub fn minter(env: Env) -> Address {
        extend_instance(&env);
        read_minter(&env)
    }

    pub fn total_supply(env: Env) -> i128 {
        extend_instance(&env);
        read_supply(&env)
    }

    pub fn admin(env: Env) -> Address {
        extend_instance(&env);
        read_administrator(&env)
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = read_administrator(&env);
        admin.require_auth();
        extend_instance(&env);
        write_administrator(&env, &new_admin);
        SetAdmin { admin, new_admin }.publish(&env);
    }

    /// Issues shares to an allowlisted recipient. The vault contract is the
    /// minter. The vault address itself does not have to be allowlisted,
    /// because mint does not transfer an existing balance.
    pub fn mint(env: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        read_minter(&env).require_auth();
        extend_instance(&env);
        require_recipient_allowlisted(&env, &to);
        receive_balance(&env, to.clone(), amount);
        add_supply(&env, amount);
        events::MintWithAmountOnly { to, amount }.publish(&env);
    }

    /// Burns shares during `vault.withdraw`. The holder must already have
    /// authorized the vault withdrawal. This does not credit another address,
    /// and it still requires the holder to be allowlisted.
    pub fn vault_burn(env: Env, from: Address, amount: i128) {
        check_nonnegative_amount(amount);
        read_minter(&env).require_auth();
        extend_instance(&env);
        require_transfer_allowed(&env, &from, &from, amount);
        spend_balance(&env, from.clone(), amount);
        sub_supply(&env, amount);
        events::Burn { from, amount }.publish(&env);
    }
}

#[contractimpl(contracttrait)]
impl TokenInterface for ReceiptToken {
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
        let to = to_muxed.address();
        require_transfer_allowed(&env, &from, &to, amount);
        spend_balance(&env, from.clone(), amount);
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
        require_transfer_allowed(&env, &from, &to, amount);
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
        require_transfer_allowed(&env, &from, &from, amount);
        spend_balance(&env, from.clone(), amount);
        sub_supply(&env, amount);
        events::Burn { from, amount }.publish(&env);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        check_nonnegative_amount(amount);
        extend_instance(&env);
        require_transfer_allowed(&env, &from, &from, amount);
        spend_allowance(&env, from.clone(), spender, amount);
        spend_balance(&env, from.clone(), amount);
        sub_supply(&env, amount);
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
