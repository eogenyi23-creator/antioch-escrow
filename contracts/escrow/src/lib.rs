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
    set_threshold, set_token, ApprovalKey, DonorKey, MilestoneKey, State,
};

// ---------------------------------------------------------------------------
// SAC token client — wraps the Stellar Asset Contract interface.
// ---------------------------------------------------------------------------

pub use soroban_sdk::token::TokenClient as SacToken;

// ---------------------------------------------------------------------------
// TTL constants
// ---------------------------------------------------------------------------

/// Low-water TTL mark: if an entry's remaining TTL falls below this value,
/// extend it. ~1 day = 17,280 ledgers (at ~5 s per ledger).
const TTL_THRESHOLD: u32 = 17_280;

/// Target TTL after extension. ~30 days = 518,400 ledgers.
const TTL_TARGET: u32 = 518_400;

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
    // --- constructor (atomic deploy + init) --------------------------------

    /// Initialize the escrow. Called once at deploy time via `__constructor`.
    ///
    /// Using `__constructor` makes deploy and initialization atomic: the
    /// contract cannot be front-run between deploy and init because there is
    /// no separate init call.
    ///
    /// Validates:
    /// - `threshold` in 1..=N (where N = number of approvers)
    /// - no duplicate approvers
    /// - at least one milestone, every amount > 0
    /// - `expiry_ledger` strictly in the future (`> env.ledger().sequence()`)
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] if called more than once (should never
    ///   happen via `__constructor`, but guarded for safety).
    /// - [`Error::InvalidThreshold`] if threshold is 0 or > N.
    /// - [`Error::DuplicateApprover`] if any approver appears more than once.
    /// - [`Error::InvalidMilestone`] if milestones is empty.
    /// - [`Error::AmountMustBePositive`] if any milestone amount ≤ 0.
    /// - [`Error::InvalidExpiry`] if `expiry_ledger ≤ env.ledger().sequence()`.
    pub fn __constructor(
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

        // Use InvalidExpiry (not NotExpired) for a bad expiry at construction.
        if expiry_ledger <= env.ledger().sequence() {
            soroban_sdk::panic_with_error!(env, Error::InvalidExpiry);
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
    ///
    /// Rejects if:
    /// - expired or completed
    /// - `amount ≤ 0`
    /// - funding would push `funded_total` above the sum of all milestone
    ///   amounts ([`Error::FundingCapExceeded`])
    ///
    /// # Errors
    /// - [`Error::Expired`] / [`Error::AlreadyReleased`] if escrow is not Active.
    /// - [`Error::AmountMustBePositive`] if `amount ≤ 0`.
    /// - [`Error::FundingCapExceeded`] if `funded_total + amount > milestone_sum`.
    /// - [`Error::Overflow`] on arithmetic overflow.
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

        // Enforce funding cap: funded_total + amount must not exceed
        // the sum of all milestone amounts.
        let milestones = get_milestones(&env);
        let mut milestone_sum: i128 = 0;
        for i in 0..milestones.len() {
            milestone_sum = milestone_sum
                .checked_add(milestones.get(i).unwrap())
                .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::Overflow));
        }
        let prev_funded = get_funded_total(&env);
        let new_funded = prev_funded
            .checked_add(amount)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::Overflow));
        if new_funded > milestone_sum {
            soroban_sdk::panic_with_error!(env, Error::FundingCapExceeded);
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
        set_funded_total(&env, new_funded);

        events::fund(&env, &donor, amount);
    }

    // --- approval ----------------------------------------------------------

    /// Record an approval from `approver` for milestone `index`.
    ///
    /// When approvals reach the threshold, attempt to release automatically.
    /// If funds are short, the approval is kept but the milestone stays pending.
    ///
    /// # Errors
    /// - [`Error::Expired`] / [`Error::AlreadyReleased`] if escrow is not Active.
    /// - [`Error::InvalidMilestone`] if index is out of bounds.
    /// - [`Error::AlreadyReleased`] if the milestone was already released.
    /// - [`Error::Unauthorized`] if approver is not in the fixed set.
    /// - [`Error::AlreadyApproved`] if this approver already approved this milestone.
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
    ///
    /// Callable by anyone. Respects the in-order rule. The beneficiary is fixed
    /// at construction, so permissionless release adds no trust risk.
    ///
    /// # Errors
    /// - [`Error::Expired`] if the escrow has expired.
    /// - [`Error::AlreadyReleased`] if this milestone was already released.
    /// - [`Error::OutOfOrder`] if milestone n-1 has not yet been released.
    /// - [`Error::ThresholdNotMet`] if insufficient approvals recorded.
    /// - [`Error::InsufficientFunds`] if unreleased balance < milestone amount.
    pub fn release_milestone(env: Env, index: u32) {
        match Self::try_release(&env, index) {
            Ok(()) => {}
            Err(e) => soroban_sdk::panic_with_error!(env, e),
        }
    }

    /// Attempt to release milestone `index`. Returns `Ok(())` on success or
    /// `Err(Error)` on failure. Does not panic — the caller decides.
    ///
    /// State mutation (released flag and released_total) is committed **before**
    /// the token transfer to prevent double-release even if the transfer reverts.
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

        // Threshold met? Use ThresholdNotMet (not InsufficientFunds) for this case.
        let threshold = get_threshold(env);
        let count = get_approval_count(env, index);
        if count < threshold {
            return Err(Error::ThresholdNotMet);
        }

        // Funds sufficient?
        let amount = milestones.get(index).unwrap();
        let funded = get_funded_total(env);
        let released = get_released_total(env);
        let unreleased = funded.checked_sub(released).ok_or(Error::Overflow)?;
        if unreleased < amount {
            return Err(Error::InsufficientFunds);
        }

        // Commit state changes BEFORE the token transfer.
        // This prevents a re-entrant or reverted transfer from allowing a
        // second release attempt.
        set_milestone_released(env, index, true);
        let new_released = released.checked_add(amount).ok_or(Error::Overflow)?;
        set_released_total(env, new_released);

        // Transfer to beneficiary (after state is committed).
        let token_addr = get_token(env);
        let sac = SacToken::new(env, &token_addr);
        let beneficiary = get_beneficiary(env);
        let contract_addr = env.current_contract_address();
        sac.transfer(&contract_addr, &beneficiary, &amount);

        events::release(env, index, amount, &beneficiary);

        Ok(())
    }

    // --- refund ------------------------------------------------------------

    /// Refund a donor's pro-rata share of the unreleased balance after expiry.
    ///
    /// Each donor can refund once. Refund math:
    /// `refund = contribution × (funded_total − released_total) ÷ funded_total`
    ///
    /// Integer division rounds down; dust stays in the contract.
    ///
    /// # Errors
    /// - [`Error::NotExpired`] if the escrow has not yet expired.
    /// - [`Error::NothingToRefund`] if the donor has zero contribution or
    ///   has already refunded.
    /// - [`Error::Overflow`] on arithmetic overflow.
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
    ///
    /// # Errors
    /// - [`Error::InvalidMilestone`] if `index` is out of bounds.
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
    ///
    /// # Errors
    /// - [`Error::InvalidMilestone`] if `index` is out of bounds.
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

    /// Extend storage TTL for instance data and per-milestone entries starting
    /// from `start_index`.
    ///
    /// Ledger timing (at ~5 s per ledger):
    /// - 1 day  ≈ 17,280 ledgers
    /// - 30 days ≈ 518,400 ledgers
    ///
    /// Entries are only extended if their current TTL is below
    /// [`TTL_THRESHOLD`] (1 day). They are extended to [`TTL_TARGET`] (30 days).
    ///
    /// Call this periodically (e.g. monthly) to keep a long-lived escrow from
    /// being archived. For donor-specific entries, call [`maintain_donor_ttl`].
    ///
    /// [`maintain_donor_ttl`]: Self::maintain_donor_ttl
    pub fn maintain_ttl(env: Env, start_index: u32) {
        // Bump instance storage (holds config, totals, approvers, etc.).
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_TARGET);

        let milestones = get_milestones(&env);
        let count = milestones.len();

        for i in start_index..count {
            // Bump released flag — only if the entry exists.
            let key_released = MilestoneKey::Released(i);
            if env.storage().persistent().has(&key_released) {
                env.storage()
                    .persistent()
                    .extend_ttl(&key_released, TTL_THRESHOLD, TTL_TARGET);
            }

            // Bump approval count — only if the entry exists.
            let key_count = MilestoneKey::ApprovalCount(i);
            if env.storage().persistent().has(&key_count) {
                env.storage()
                    .persistent()
                    .extend_ttl(&key_count, TTL_THRESHOLD, TTL_TARGET);
            }
        }
    }

    /// Extend storage TTL for a single donor's contribution and refund entries.
    ///
    /// Call this for each active donor when maintaining a long-lived escrow.
    /// Entries are only extended if they exist and are below [`TTL_THRESHOLD`].
    pub fn maintain_donor_ttl(env: Env, donor: Address) {
        let key_contrib = DonorKey::Contribution(donor.clone());
        if env.storage().persistent().has(&key_contrib) {
            env.storage()
                .persistent()
                .extend_ttl(&key_contrib, TTL_THRESHOLD, TTL_TARGET);
        }

        let key_refunded = DonorKey::Refunded(donor);
        if env.storage().persistent().has(&key_refunded) {
            env.storage()
                .persistent()
                .extend_ttl(&key_refunded, TTL_THRESHOLD, TTL_TARGET);
        }
    }
}
