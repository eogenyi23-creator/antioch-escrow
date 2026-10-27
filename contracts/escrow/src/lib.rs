#![no_std]

mod error;
mod events;
mod storage;
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use error::Error;
use storage::{
    derive_state, get_approval_count, get_approvers, get_beneficiary, get_donor_contribution,
    get_funded_total, get_milestones, get_released_total, get_threshold, get_token, has_approved,
    is_donor_refunded, is_initialized, is_milestone_released, set_approval_count, set_approved,
    set_approvers, set_beneficiary, set_donor_contribution, set_donor_refunded, set_expiry_ledger,
    set_funded_total, set_initialized, set_milestone_released, set_milestones, set_released_total,
    set_threshold, set_token, ApprovalKey, State,
};

// ---------------------------------------------------------------------------
// SAC token client — wraps the Stellar Asset Contract interface.
// ---------------------------------------------------------------------------

pub use soroban_sdk::token::TokenClient as SacToken;

// ---------------------------------------------------------------------------
// Contract struct
// ---------------------------------------------------------------------------

#[contract]
pub struct EscrowContract;

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl EscrowContract {
    // --- initialization ---------------------------------------------------

    /// Initialize the escrow. Callable once.
    ///
    /// Validates:
    /// - `threshold` in 1..=N (where N = number of approvers)
    /// - no duplicate approvers
    /// - at least one milestone
    /// - every milestone amount > 0
    /// - `expiry_ledger` in the future
    pub fn init(
        env: Env,
        approvers: Vec<Address>,
        threshold: u32,
        beneficiary: Address,
        token: Address,
        milestones: Vec<i128>,
        expiry_ledger: u32,
    ) {
        if is_initialized(&env) {
            soroban_sdk::panic_with_error!(env, Error::AlreadyInitialized);
        }

        let n = approvers.len();
        if n == 0 {
            soroban_sdk::panic_with_error!(env, Error::InvalidThreshold);
        }
        if threshold == 0 || threshold > n {
            soroban_sdk::panic_with_error!(env, Error::InvalidThreshold);
        }

        // Check for duplicate approvers.
        for i in 0..n {
            for j in (i + 1)..n {
                if approvers.get(i).unwrap() == approvers.get(j).unwrap() {
                    soroban_sdk::panic_with_error!(env, Error::DuplicateApprover);
                }
            }
        }

        if milestones.is_empty() {
            soroban_sdk::panic_with_error!(env, Error::InvalidMilestone);
        }

        for i in 0..milestones.len() {
            let amount = milestones.get(i).unwrap();
            if amount <= 0 {
                soroban_sdk::panic_with_error!(env, Error::AmountMustBePositive);
            }
        }

        if expiry_ledger <= env.ledger().sequence() {
            soroban_sdk::panic_with_error!(env, Error::NotExpired);
        }

        set_initialized(&env);
        set_approvers(&env, &approvers);
        set_threshold(&env, threshold);
        set_beneficiary(&env, &beneficiary);
        set_token(&env, &token);
        set_milestones(&env, &milestones);
        set_expiry_ledger(&env, expiry_ledger);
        set_funded_total(&env, 0);
        set_released_total(&env, 0);

        events::init(&env, &beneficiary, &token, threshold, milestones.len());
    }

    // --- funding -----------------------------------------------------------

    /// Deposit tokens from `donor` into the contract.
    /// Rejects if expired or completed.
    pub fn fund(env: Env, donor: Address, amount: i128) {
        donor.require_auth();

        if amount <= 0 {
            soroban_sdk::panic_with_error!(env, Error::AmountMustBePositive);
        }

        let state = derive_state(&env);
        if state == State::Expired {
            soroban_sdk::panic_with_error!(env, Error::Expired);
        }
        if state == State::Completed {
            soroban_sdk::panic_with_error!(env, Error::AlreadyReleased);
        }

        // Transfer tokens from donor to the contract.
        let token_addr = get_token(&env);
        let sac = SacToken::new(&env, &token_addr);
        let contract_addr = env.current_contract_address();
        sac.transfer(&donor, &contract_addr, &amount);

        // Record contribution and update totals.
        let prev_contribution = get_donor_contribution(&env, &donor);
        let new_contribution = prev_contribution
            .checked_add(amount)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::Overflow));
        set_donor_contribution(&env, &donor, new_contribution);

        let prev_funded = get_funded_total(&env);
        let new_funded = prev_funded
            .checked_add(amount)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::Overflow));
        set_funded_total(&env, new_funded);

        events::fund(&env, &donor, amount);
    }

    // --- approval ----------------------------------------------------------

    /// Record an approval from `approver` for milestone `index`.
    /// When approvals reach the threshold, attempt to release automatically.
    /// If funds are short, the approval is kept but the milestone stays pending.
    pub fn approve_milestone(env: Env, approver: Address, index: u32) {
        approver.require_auth();

        let state = derive_state(&env);
        if state == State::Expired {
            soroban_sdk::panic_with_error!(env, Error::Expired);
        }
        if state == State::Completed {
            soroban_sdk::panic_with_error!(env, Error::AlreadyReleased);
        }

        // Validate index.
        let milestones = get_milestones(&env);
        if index >= milestones.len() {
            soroban_sdk::panic_with_error!(env, Error::InvalidMilestone);
        }

        // Milestone must not already be released.
        if is_milestone_released(&env, index) {
            soroban_sdk::panic_with_error!(env, Error::AlreadyReleased);
        }

        // Approver must be in the set.
        let approvers = get_approvers(&env);
        let mut found = false;
        for i in 0..approvers.len() {
            if approvers.get(i).unwrap() == approver {
                found = true;
                break;
            }
        }
        if !found {
            soroban_sdk::panic_with_error!(env, Error::Unauthorized);
        }

        // Same approver cannot approve twice.
        let key = ApprovalKey {
            milestone: index,
            approver: approver.clone(),
        };
        if has_approved(&env, &key) {
            soroban_sdk::panic_with_error!(env, Error::AlreadyApproved);
        }
        set_approved(&env, &key, true);

        // Increment approval count.
        let count = get_approval_count(&env, index);
        let new_count = count + 1;
        set_approval_count(&env, index, new_count);

        events::approve(&env, &approver, index);

        // Attempt automatic release if threshold reached.
        let threshold = get_threshold(&env);
        if new_count >= threshold {
            // Best-effort release; do not fail the approval if funds are short.
            let _ = Self::try_release(&env, index);
        }
    }

    // --- release -----------------------------------------------------------

    /// Release a milestone whose threshold is met and whose funds are now sufficient.
    /// Callable by anyone. Respects the in-order rule.
    pub fn release_milestone(env: Env, index: u32) {
        match Self::try_release(&env, index) {
            Ok(()) => {}
            Err(e) => soroban_sdk::panic_with_error!(env, e),
        }
    }

    /// Attempt to release milestone `index`. Returns `Ok(())` on success or
    /// `Err(Error)` on failure. Does not panic — the caller decides what to do.
    fn try_release(env: &Env, index: u32) -> Result<(), Error> {
        let state = derive_state(env);
        if state == State::Expired {
            return Err(Error::Expired);
        }
        if state == State::Completed {
            return Err(Error::AlreadyReleased);
        }

        let milestones = get_milestones(env);
        if index >= milestones.len() {
            return Err(Error::InvalidMilestone);
        }

        // Already released?
        if is_milestone_released(env, index) {
            return Err(Error::AlreadyReleased);
        }

        // In-order check: milestone n requires milestone n-1 to be released.
        if index > 0 && !is_milestone_released(env, index - 1) {
            return Err(Error::OutOfOrder);
        }

        // Threshold met?
        let threshold = get_threshold(env);
        let count = get_approval_count(env, index);
        if count < threshold {
            return Err(Error::InsufficientFunds);
        }

        // Funds sufficient?
        let amount = milestones.get(index).unwrap();
        let funded = get_funded_total(env);
        let released = get_released_total(env);
        // unreleased = funded - released (checked)
        let unreleased = funded.checked_sub(released).ok_or(Error::Overflow)?;
        if unreleased < amount {
            return Err(Error::InsufficientFunds);
        }

        // Transfer to beneficiary.
        let token_addr = get_token(env);
        let sac = SacToken::new(env, &token_addr);
        let beneficiary = get_beneficiary(env);
        let contract_addr = env.current_contract_address();
        sac.transfer(&contract_addr, &beneficiary, &amount);

        // Mark released and update totals.
        set_milestone_released(env, index, true);
        let new_released = released.checked_add(amount).ok_or(Error::Overflow)?;
        set_released_total(env, new_released);

        events::release(env, index, amount, &beneficiary);

        Ok(())
    }

    // --- refund ------------------------------------------------------------

    /// Refund a donor's pro-rata share of the unreleased balance after expiry.
    /// Each donor can refund once.
    pub fn refund(env: Env, donor: Address) -> i128 {
        donor.require_auth();

        let state = derive_state(&env);
        if state != State::Expired {
            soroban_sdk::panic_with_error!(env, Error::NotExpired);
        }

        // Already refunded?
        if is_donor_refunded(&env, &donor) {
            soroban_sdk::panic_with_error!(env, Error::NothingToRefund);
        }

        let contribution = get_donor_contribution(&env, &donor);
        if contribution <= 0 {
            soroban_sdk::panic_with_error!(env, Error::NothingToRefund);
        }

        let funded = get_funded_total(&env);
        let released = get_released_total(&env);
        if funded == 0 {
            soroban_sdk::panic_with_error!(env, Error::NothingToRefund);
        }

        // refund = contribution * (funded - released) / funded
        let unreleased = funded
            .checked_sub(released)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::Overflow));

        let refund_amount = contribution
            .checked_mul(unreleased)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::Overflow))
            / funded;

        if refund_amount <= 0 {
            soroban_sdk::panic_with_error!(env, Error::NothingToRefund);
        }

        // Transfer refund to donor.
        let token_addr = get_token(&env);
        let sac = SacToken::new(&env, &token_addr);
        let contract_addr = env.current_contract_address();
        sac.transfer(&contract_addr, &donor, &refund_amount);

        // Mark donor as refunded.
        set_donor_refunded(&env, &donor, true);

        events::refund(&env, &donor, refund_amount);

        refund_amount
    }

    // --- views -------------------------------------------------------------

    /// Get the current state of the escrow.
    pub fn state(env: Env) -> State {
        derive_state(&env)
    }

    /// Get the total amount funded.
    pub fn funded_total(env: Env) -> i128 {
        get_funded_total(&env)
    }

    /// Get the total amount released to the beneficiary.
    pub fn released_total(env: Env) -> i128 {
        get_released_total(&env)
    }

    /// Get milestone amount and released flag by index.
    pub fn milestone(env: Env, index: u32) -> (i128, bool) {
        let milestones = get_milestones(&env);
        if index >= milestones.len() {
            soroban_sdk::panic_with_error!(env, Error::InvalidMilestone);
        }
        let amount = milestones.get(index).unwrap();
        let released = is_milestone_released(&env, index);
        (amount, released)
    }

    /// Get the number of milestones.
    pub fn milestone_count(env: Env) -> u32 {
        get_milestones(&env).len()
    }

    /// Get the number of approvals recorded for a milestone.
    pub fn approvals(env: Env, index: u32) -> u32 {
        let milestones = get_milestones(&env);
        if index >= milestones.len() {
            soroban_sdk::panic_with_error!(env, Error::InvalidMilestone);
        }
        get_approval_count(&env, index)
    }

    /// Get a donor's total contribution.
    pub fn donor_contribution(env: Env, donor: Address) -> i128 {
        get_donor_contribution(&env, &donor)
    }

    // --- TTL ---------------------------------------------------------------

    /// Extend storage TTL for persistent entries starting from `start_index`.
    /// Helps long-lived escrows avoid archival.
    ///
    /// `threshold` is the low-water mark: if TTL is below this, extend to `extend_to`.
    /// Both are in number of ledgers. ~30 days = 51840 ledgers (at ~5s per ledger).
    pub fn maintain_ttl(env: Env, start_index: u32) {
        let milestones = get_milestones(&env);
        let count = milestones.len();

        // Bump instance data.
        env.storage().instance().extend_ttl(1000, 51840);

        // Bump milestone and approval entries.
        for i in start_index..count {
            // Bump released flag.
            let key_released = storage::MilestoneKey::Released(i);
            env.storage()
                .persistent()
                .extend_ttl(&key_released, 1000, 51840);

            // Bump approval count.
            let key_count = storage::MilestoneKey::ApprovalCount(i);
            env.storage()
                .persistent()
                .extend_ttl(&key_count, 1000, 51840);
        }
    }
}
