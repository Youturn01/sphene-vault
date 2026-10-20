use soroban_sdk::{testutils::Address as _, Address, Env, String};
use sphene_compliance::{Compliance, ComplianceClient};
use sphene_mock_usdc::{MockUsdc, MockUsdcClient};
use sphene_receipt_token::{ReceiptToken, ReceiptTokenClient};
use sphene_vault::{Vault, VaultClient};

/// Asset base units per one whole share when one share unit equals one asset unit.
pub const PAR_NAV: i128 = 10_000_000;

pub struct Contracts {
    pub admin: Address,
    pub alice: Address,
    pub bob: Address,
    pub carol: Address,
    pub asset: Address,
    pub receipt: Address,
    pub compliance: Address,
    pub vault: Address,
}

pub fn deploy(env: &Env) -> Contracts {
    let admin = Address::generate(env);
    let alice = Address::generate(env);
    let bob = Address::generate(env);
    let carol = Address::generate(env);

    let asset = env.register(
        MockUsdc,
        (
            admin.clone(),
            7_u32,
            String::from_str(env, "Mock USD Coin"),
            String::from_str(env, "mUSDC"),
        ),
    );
    let compliance = env.register(Compliance, (admin.clone(),));
    let receipt = env.register(
        ReceiptToken,
        (
            admin.clone(),
            7_u32,
            String::from_str(env, "Sphene Receipt"),
            String::from_str(env, "sphRWA"),
        ),
    );
    let vault = env.register(Vault, (admin.clone(),));

    Contracts {
        admin,
        alice,
        bob,
        carol,
        asset,
        receipt,
        compliance,
        vault,
    }
}

pub fn configure(env: &Env, contracts: &Contracts, nav: i128) {
    let receipt = ReceiptTokenClient::new(env, &contracts.receipt);
    receipt.set_compliance(&contracts.compliance);
    receipt.set_minter(&contracts.vault);
    VaultClient::new(env, &contracts.vault).initialize(
        &contracts.asset,
        &contracts.receipt,
        &nav,
    );
}

pub fn allow(env: &Env, contracts: &Contracts, address: &Address) {
    ComplianceClient::new(env, &contracts.compliance).add_to_allowlist(address);
}

pub fn mint_asset(env: &Env, contracts: &Contracts, to: &Address, amount: i128) {
    MockUsdcClient::new(env, &contracts.asset).mint(to, &amount);
}
