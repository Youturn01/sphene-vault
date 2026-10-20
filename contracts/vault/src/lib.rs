#![no_std]

//! Fixed-NAV vault.
//!
//! Share accounting, with `scale = 10^receipt_decimals`:
//!
//! - shares minted = floor(assets_received * scale / nav)
//! - assets withdrawn = floor(shares_burned * nav / scale)
//!
//! `nav` is an integer number of asset base units per `scale` share base
//! units. It is set once at initialization and never updated from a price
//! feed. Integer division rounds toward zero. For the positive amounts this
//! contract accepts, that rounds down and leaves any remainder in the vault.
//!
//! A deposit that would mint zero shares is rejected. A withdrawal that would
//! pay zero assets is rejected. Failed calls revert, so asset balances and
//! share balances stay aligned.

use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, token::TokenClient, Address, Env,
};

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[derive(Clone)]
#[soroban_sdk::contracttype]
enum DataKey {
    Admin,
    Asset,
    Receipt,
    Nav,
    Scale,
    TotalAssets,
    Locked,
}

struct State {
    asset: Address,
    receipt: Address,
    nav: i128,
    scale: i128,
    total_assets: i128,
}

/// Client for the receipt-token methods the vault calls. Keeping the client
/// local avoids linking the receipt contract into this Wasm.
#[contractclient(name = "ReceiptClient")]
trait ReceiptInterface {
    fn balance(env: Env, id: Address) -> i128;
    fn decimals(env: Env) -> u32;
    fn total_supply(env: Env) -> i128;
    fn minter(env: Env) -> Address;
    fn compliance(env: Env) -> Address;
    fn mint(env: Env, to: Address, amount: i128);
    fn vault_burn(env: Env, from: Address, amount: i128);
}

#[contract]
pub struct Vault;

#[contractevent]
pub struct Deposited {
    #[topic]
    pub depositor: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractevent]
pub struct Withdrawn {
    #[topic]
    pub holder: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractimpl]
impl Vault {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        extend_instance(&env);
    }

    /// Sets the asset, the receipt token, and the fixed NAV. Callable once,
    /// and only by the admin. The vault must already be the receipt minter.
    pub fn initialize(env: Env, asset: Address, receipt: Address, nav: i128) {
        read_admin(&env).require_auth();
        if initialized(&env) {
            panic!("vault already initialized");
        }
        if nav <= 0 {
            panic!("nav must be positive");
        }
        if asset == receipt {
            panic!("asset and receipt must differ");
        }
        extend_instance(&env);

        let asset_token = TokenClient::new(&env, &asset);
        let _asset_decimals = asset_token.decimals();

        let shares = ReceiptClient::new(&env, &receipt);
        let _compliance = shares.compliance();
        let minter = shares.minter();
        if minter != env.current_contract_address() {
            panic!("vault is not receipt minter");
        }
        let scale = scale_from_decimals(shares.decimals());

        env.storage().instance().set(&DataKey::Asset, &asset);
        env.storage().instance().set(&DataKey::Receipt, &receipt);
        env.storage().instance().set(&DataKey::Nav, &nav);
        env.storage().instance().set(&DataKey::Scale, &scale);
        env.storage().instance().set(&DataKey::TotalAssets, &0_i128);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = read_admin(&env);
        admin.require_auth();
        extend_instance(&env);
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    /// Pulls `amount` of the configured asset from `from` and mints shares.
    /// Returns the number of share base units minted.
    pub fn deposit(env: Env, from: Address, amount: i128) -> i128 {
        from.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }
        let state = read_state(&env);
        enter(&env);
        extend_instance(&env);

        let shares = shares_for_assets(amount, state.nav, state.scale);
        if shares <= 0 {
            panic!("deposit mints zero shares");
        }

        let vault = env.current_contract_address();
        let asset = TokenClient::new(&env, &state.asset);
        let before = asset.balance(&vault);
        asset.transfer(&from, &vault, &amount);
        let after = asset.balance(&vault);
        let received = after
            .checked_sub(before)
            .expect("asset balance decreased on deposit");
        if received <= 0 {
            panic!("no assets received");
        }
        let shares = shares_for_assets(received, state.nav, state.scale);
        if shares <= 0 {
            panic!("deposit mints zero shares");
        }

        let total_assets = state
            .total_assets
            .checked_add(received)
            .expect("total assets overflow");
        env.storage()
            .instance()
            .set(&DataKey::TotalAssets, &total_assets);

        ReceiptClient::new(&env, &state.receipt).mint(&from, &shares);
        Deposited {
            depositor: from,
            assets: received,
            shares,
        }
        .publish(&env);
        exit(&env);
        shares
    }

    /// Burns `shares` from `to` and pays the fixed-NAV amount of the asset.
    /// The burn runs before the asset transfer. Both calls are in this
    /// invocation, so a failed payment reverts the burn.
    pub fn withdraw(env: Env, to: Address, shares: i128) -> i128 {
        to.require_auth();
        if shares <= 0 {
            panic!("shares must be positive");
        }
        let state = read_state(&env);
        enter(&env);
        extend_instance(&env);

        let receipt = ReceiptClient::new(&env, &state.receipt);
        let balance = receipt.balance(&to);
        if balance < shares {
            panic!("insufficient shares");
        }
        let assets = assets_for_shares(shares, state.nav, state.scale);
        if assets <= 0 {
            panic!("withdrawal pays zero assets");
        }
        if assets > state.total_assets {
            panic!("insufficient vault assets");
        }

        let vault = env.current_contract_address();
        let asset = TokenClient::new(&env, &state.asset);
        let available = asset.balance(&vault);
        if assets > available {
            panic!("insufficient vault assets");
        }

        receipt.vault_burn(&to, &shares);
        let total_assets = state
            .total_assets
            .checked_sub(assets)
            .expect("total assets underflow");
        env.storage()
            .instance()
            .set(&DataKey::TotalAssets, &total_assets);
        asset.transfer(&vault, &to, &assets);

        Withdrawn {
            holder: to,
            assets,
            shares,
        }
        .publish(&env);
        exit(&env);
        assets
    }

    pub fn preview_deposit(env: Env, assets: i128) -> i128 {
        if assets < 0 {
            panic!("amount must be non-negative");
        }
        if assets == 0 {
            return 0;
        }
        let state = read_state(&env);
        shares_for_assets(assets, state.nav, state.scale)
    }

    pub fn preview_withdraw(env: Env, shares: i128) -> i128 {
        if shares < 0 {
            panic!("shares must be non-negative");
        }
        if shares == 0 {
            return 0;
        }
        let state = read_state(&env);
        assets_for_shares(shares, state.nav, state.scale)
    }

    pub fn shares(env: Env, user: Address) -> i128 {
        let state = read_state(&env);
        ReceiptClient::new(&env, &state.receipt).balance(&user)
    }

    pub fn total_shares(env: Env) -> i128 {
        let state = read_state(&env);
        ReceiptClient::new(&env, &state.receipt).total_supply()
    }

    pub fn total_assets(env: Env) -> i128 {
        read_state(&env).total_assets
    }

    pub fn asset_balance(env: Env) -> i128 {
        let state = read_state(&env);
        TokenClient::new(&env, &state.asset).balance(&env.current_contract_address())
    }

    pub fn nav(env: Env) -> i128 {
        read_state(&env).nav
    }

    pub fn scale(env: Env) -> i128 {
        read_state(&env).scale
    }

    pub fn asset(env: Env) -> Address {
        read_state(&env).asset
    }

    pub fn receipt(env: Env) -> Address {
        read_state(&env).receipt
    }

    pub fn admin(env: Env) -> Address {
        extend_instance(&env);
        read_admin(&env)
    }

    pub fn initialized(env: Env) -> bool {
        extend_instance(&env);
        initialized(&env)
    }
}

fn shares_for_assets(assets: i128, nav: i128, scale: i128) -> i128 {
    assets
        .checked_mul(scale)
        .expect("share calculation overflow")
        .checked_div(nav)
        .expect("invalid nav")
}

fn assets_for_shares(shares: i128, nav: i128, scale: i128) -> i128 {
    shares
        .checked_mul(nav)
        .expect("asset calculation overflow")
        .checked_div(scale)
        .expect("invalid scale")
}

fn scale_from_decimals(decimals: u32) -> i128 {
    if decimals > 18 {
        panic!("decimals too large");
    }
    let mut scale: i128 = 1;
    let mut i = 0;
    while i < decimals {
        scale = scale.checked_mul(10).expect("scale overflow");
        i += 1;
    }
    scale
}

fn initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Asset)
}

fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("vault is not initialized")
}

fn read_state(env: &Env) -> State {
    if !initialized(env) {
        panic!("vault is not initialized");
    }
    State {
        asset: env.storage().instance().get(&DataKey::Asset).unwrap(),
        receipt: env.storage().instance().get(&DataKey::Receipt).unwrap(),
        nav: env.storage().instance().get(&DataKey::Nav).unwrap(),
        scale: env.storage().instance().get(&DataKey::Scale).unwrap(),
        total_assets: env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap(),
    }
}

fn enter(env: &Env) {
    if env.storage().instance().get(&DataKey::Locked).unwrap_or(false) {
        panic!("reentrant call");
    }
    env.storage().instance().set(&DataKey::Locked, &true);
}

fn exit(env: &Env) {
    env.storage().instance().set(&DataKey::Locked, &false);
}

fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}
