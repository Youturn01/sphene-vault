# Sphene Vault

**Compliance-Gated Vault Infrastructure on Stellar**

Sphene Vault is a blockchain project designed to provide secure infrastructure for tokenized real-world assets (RWAs) on the Stellar network. It combines a digital vault, share accounting, receipt tokens, and programmable transfer restrictions to support controlled participation in tokenized asset applications.

## Project Overview

Access to tokenized assets such as treasury bills and property can be challenging for African users due to limited local access, offshore platforms, and regulatory and security requirements.

Sphene Vault aims to address part of this challenge by developing reusable smart contract infrastructure that enables controlled asset participation through transparent, blockchain-based rules.

The initial prototype will be deployed on Stellar Testnet and will demonstrate vault operations, receipt-token issuance, and compliance-controlled transfers.

## Key Features

- **Vault Core:** A Soroban smart contract supporting deposits, withdrawals, and share accounting.
- **Receipt Tokens:** SEP-41-compatible tokens representing users' shares in the vault.
- **Transfer Restrictions:** Automated checks that prevent unauthorized transfers.
- **Address Allowlisting:** Administrators can approve or remove wallet addresses.
- **Smart Contract Integration:** Coordination between the vault, receipt token, and compliance contract.
- **Transparent Verification:** On-chain transactions provide evidence of contract activity.
- **Open-Source Development:** Public source code, documentation, deployment instructions, and testing resources.

## How It Works

1. **Deposit:** A user deposits the supported token into the vault.
2. **Share Accounting:** The vault calculates and records the user's shares according to its accounting rules.
3. **Receipt Token Issuance:** The user receives tokens representing their participation in the vault.
4. **Compliance Check:** When a receipt token transfer is initiated, the compliance contract checks the applicable allowlist rules.
5. **Transfer Authorization:** Transfers that satisfy the rules proceed, while unauthorized transfers are rejected.
6. **Address Approval:** An administrator can approve a previously unauthorized address, allowing eligible transfers to proceed.

## Technology Stack

- **Blockchain:** Stellar
- **Smart Contract Platform:** Soroban
- **Programming Language:** Rust
- **Token Standard:** SEP-41
- **Development Tools:** Stellar CLI and Soroban tooling
- **Deployment Environment:** Stellar Testnet

## System Architecture

Sphene Vault consists of three main components:

### Vault Contract

Manages deposits, withdrawals, and share accounting. It serves as the core component for tracking user participation.

### Receipt Token Contract

Implements the receipt-token functionality and checks compliance rules before allowing applicable token transfers.

### Compliance Contract

Maintains the address allowlist, manages transfer eligibility, and restricts administrative actions to authorized accounts.

These components work together to demonstrate how tokenized asset infrastructure can incorporate programmable access controls.

## Security Considerations

- Administrative functions must be protected against unauthorized access.
- All applicable token transfer paths must enforce compliance checks.
- Share calculations must handle precision, rounding, and accounting edge cases.
- Failed transactions must not leave inconsistent balances.
- Contract interactions and permissions must be tested thoroughly.
- The prototype requires appropriate security review before any production deployment.

An address allowlist is a technical access-control mechanism and does not, by itself, establish regulatory compliance or identity verification.

## Project Scope

The initial version focuses on:

- Vault deposits and withdrawals.
- Share accounting at a fixed starting net asset value.
- Receipt-token issuance and transfers.
- Compliance checks and manual address allowlisting.
- Stellar Testnet deployment and integration testing.
- Public documentation and a demonstration of the complete workflow.

The initial version does not include live asset pricing, automated KYC integration, a frontend, mainnet deployment, independent security auditing, real-world asset issuer onboarding, or a redemption queue.

## Development Roadmap

1. Develop and test the vault contract.
2. Implement the compliance contract and receipt token.
3. Integrate the contracts and validate transfer restrictions.
4. Deploy the prototype to Stellar Testnet.
5. Verify deposit, withdrawal, and transfer scenarios.
6. Publish the source code, documentation, and demonstration.
7. Explore future improvements to support more advanced tokenized asset applications.
