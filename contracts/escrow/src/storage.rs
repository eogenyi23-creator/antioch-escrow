use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::error::Error;

// ---------------------------------------------------------------------------
// Contract state
// ---------------------------------------------------------------------------

/// The lifecycle state of the escrow, derived from storage, never stored
/// directly as a single field.
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Uninitialized,
    Active,
    Completed,
    Expired,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Keys for instance-level (singleton) data.
#[contracttype]
pub enum DataKey {
    /// Whether `init` has run.
    Init,
    /// The fixed approver set.
    Approvers,
    /// The M-of-N threshold.
    Threshold,
    /// The fixed beneficiary.
    Beneficiary,
    /// The fixed SAC token address.
    Token,
    /// The milestone amounts (fixed at init).
    Milestones,
    /// The ledger after which the escrow expires.
    ExpiryLedger,
    /// Total tokens ever funded.
    FundedTotal,
    /// Total tokens ever released to the beneficiary.
    ReleasedTotal,
}

/// Keys for per-milestone data.
#[contracttype]
pub enum MilestoneKey {
    /// Whether milestone `index` has been released.
    Released(u32),
    /// Number of approvals recorded for milestone `index`.
    ApprovalCount(u32),
}

/// Keys for per-approver, per-milestone data.
#[contracttype]
pub struct ApprovalKey {
    pub milestone: u32,
    pub approver: Address,
}

/// Keys for per-donor data.
#[contracttype]
pub enum DonorKey {
    /// A donor's total contribution.
    Contribution(Address),
    /// Whether a donor has already refunded.
    Refunded(Address),
}

// ---------------------------------------------------------------------------
// Instance data helpers
// ---------------------------------------------------------------------------

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Init)
}

pub fn set_initialized(env: &Env) {
    env.storage().instance().set(&DataKey::Init, &true);
}

pub fn get_approvers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Approvers)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_approvers(env: &Env, approvers: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Approvers, approvers);
}

pub fn get_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::Threshold)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

pub fn get_beneficiary(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Beneficiary)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_beneficiary(env: &Env, beneficiary: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::Beneficiary, beneficiary);
}

pub fn get_token(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Token)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_token(env: &Env, token: &Address) {
    env.storage().instance().set(&DataKey::Token, token);
}

pub fn get_milestones(env: &Env) -> Vec<i128> {
    env.storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_milestones(env: &Env, milestones: &Vec<i128>) {
    env.storage()
        .instance()
        .set(&DataKey::Milestones, milestones);
}

pub fn get_expiry_ledger(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ExpiryLedger)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_expiry_ledger(env: &Env, ledger: u32) {
    env.storage()
        .instance()
        .set(&DataKey::ExpiryLedger, &ledger);
}

pub fn get_funded_total(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::FundedTotal)
        .unwrap_or(0)
}

pub fn set_funded_total(env: &Env, total: i128) {
    env.storage().instance().set(&DataKey::FundedTotal, &total);
}

pub fn get_released_total(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::ReleasedTotal)
        .unwrap_or(0)
}

pub fn set_released_total(env: &Env, total: i128) {
    env.storage()
        .instance()
        .set(&DataKey::ReleasedTotal, &total);
}

// ---------------------------------------------------------------------------
// Per-milestone data
// ---------------------------------------------------------------------------

pub fn is_milestone_released(env: &Env, index: u32) -> bool {
    env.storage()
        .persistent()
        .get(&MilestoneKey::Released(index))
        .unwrap_or(false)
}

pub fn set_milestone_released(env: &Env, index: u32, released: bool) {
    env.storage()
        .persistent()
        .set(&MilestoneKey::Released(index), &released);
}

pub fn get_approval_count(env: &Env, index: u32) -> u32 {
    env.storage()
        .persistent()
        .get(&MilestoneKey::ApprovalCount(index))
        .unwrap_or(0)
}

pub fn set_approval_count(env: &Env, index: u32, count: u32) {
    env.storage()
        .persistent()
        .set(&MilestoneKey::ApprovalCount(index), &count);
}

// ---------------------------------------------------------------------------
// Per-approver, per-milestone data
// ---------------------------------------------------------------------------

pub fn has_approved(env: &Env, key: &ApprovalKey) -> bool {
    env.storage().persistent().get(key).unwrap_or(false)
}

pub fn set_approved(env: &Env, key: &ApprovalKey, approved: bool) {
    env.storage().persistent().set(key, &approved);
}

// ---------------------------------------------------------------------------
// Per-donor data
// ---------------------------------------------------------------------------

pub fn get_donor_contribution(env: &Env, donor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DonorKey::Contribution(donor.clone()))
        .unwrap_or(0)
}

pub fn set_donor_contribution(env: &Env, donor: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&DonorKey::Contribution(donor.clone()), &amount);
}

pub fn is_donor_refunded(env: &Env, donor: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DonorKey::Refunded(donor.clone()))
        .unwrap_or(false)
}

pub fn set_donor_refunded(env: &Env, donor: &Address, refunded: bool) {
    env.storage()
        .persistent()
        .set(&DonorKey::Refunded(donor.clone()), &refunded);
}

// ---------------------------------------------------------------------------
// State derivation
// ---------------------------------------------------------------------------

/// Derive the contract state from storage. Precedence:
/// `Uninitialized` > `Completed` > `Expired` > `Active`.
pub fn derive_state(env: &Env) -> State {
    if !is_initialized(env) {
        return State::Uninitialized;
    }

    // If all milestones are released, the escrow is Completed.
    let milestones = get_milestones(env);
    let count = milestones.len();
    let mut all_released = true;
    for i in 0..count {
        if !is_milestone_released(env, i) {
            all_released = false;
            break;
        }
    }
    if all_released {
        return State::Completed;
    }

    // If the current ledger has passed the expiry, the escrow is Expired.
    if env.ledger().sequence() >= get_expiry_ledger(env) {
        return State::Expired;
    }

    State::Active
}
