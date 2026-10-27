#![cfg(test)]

use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{vec, Address, Env};

use crate::storage::State;
use crate::{EscrowContract, EscrowContractClient};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct TestSetup {
    env: Env,
    client: EscrowContractClient<'static>,
    contract_id: Address,
    token: Address,
    _token_admin: Address,
    beneficiary: Address,
    approver1: Address,
    approver2: Address,
    approver3: Address,
    donor1: Address,
    donor2: Address,
    _milestones: soroban_sdk::Vec<i128>,
    expiry: u32,
}

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn make_token(env: &Env) -> (Address, Address) {
    let admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token = sac.address();
    (token, admin)
}

/// Register a new EscrowContract with constructor args, returning the client
/// and contract address. This replaces the old `client.init(...)` pattern.
fn setup(
    env: &Env,
    threshold: u32,
    milestones: soroban_sdk::Vec<i128>,
    expiry_offset: u32,
) -> TestSetup {
    let (token, token_admin) = make_token(env);

    let beneficiary = Address::generate(env);
    let approver1 = Address::generate(env);
    let approver2 = Address::generate(env);
    let approver3 = Address::generate(env);
    let donor1 = Address::generate(env);
    let donor2 = Address::generate(env);

    let approvers = vec![env, approver1.clone(), approver2.clone(), approver3.clone()];
    let expiry = env.ledger().sequence() + expiry_offset;

    // __constructor args are passed as the second argument to env.register().
    let contract_id = env.register(
        EscrowContract,
        (
            approvers,
            threshold,
            beneficiary.clone(),
            token.clone(),
            milestones.clone(),
            expiry,
        ),
    );

    let client = EscrowContractClient::new(env, &contract_id);

    TestSetup {
        env: env.clone(),
        client,
        contract_id,
        token,
        _token_admin: token_admin,
        beneficiary,
        approver1,
        approver2,
        approver3,
        donor1,
        donor2,
        _milestones: milestones,
        expiry,
    }
}

fn default_setup() -> TestSetup {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    setup(&env, 2, milestones, 1000)
}

/// Mint tokens to an address using the mock SAC token.
fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    use soroban_sdk::token::StellarAssetClient;
    let sac = StellarAssetClient::new(env, token);
    sac.mint(to, &amount);
}

// ---------------------------------------------------------------------------
// Constructor / init tests
// ---------------------------------------------------------------------------

#[test]
fn test_init_success() {
    let t = default_setup();
    assert_eq!(t.client.state(), State::Active);
    assert_eq!(t.client.milestone_count(), 2);
    assert_eq!(t.client.funded_total(), 0);
    assert_eq!(t.client.released_total(), 0);
    let (amount, released) = t.client.milestone(&0);
    assert_eq!(amount, 1000);
    assert!(!released);
    let (amount2, released2) = t.client.milestone(&1);
    assert_eq!(amount2, 2000);
    assert!(!released2);
}

/// The constructor runs exactly once at deploy time. Attempting to deploy
/// a second contract instance with invalid args (threshold=0) correctly fails,
/// proving the constructor validates eagerly. We also verify that an already-
/// initialized contract cannot be re-initialized: register a contract, then
/// register it again at the same address (which re-runs the constructor on the
/// existing storage) — this must fail with AlreadyInitialized.
#[test]
#[should_panic(expected = "#2)")] // AlreadyInitialized
fn test_constructor_cannot_run_twice() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 2, milestones.clone(), 1000);

    let approvers = vec![
        &env,
        t.approver1.clone(),
        t.approver2.clone(),
        t.approver3.clone(),
    ];
    let expiry = env.ledger().sequence() + 1000;

    // Re-register at the SAME contract address with the same constructor args.
    // The soroban test environment re-runs the constructor on the existing
    // storage, which must fail with AlreadyInitialized.
    env.register_at(
        &t.contract_id,
        EscrowContract,
        (
            approvers,
            2_u32,
            t.beneficiary.clone(),
            t.token.clone(),
            milestones,
            expiry,
        ),
    );
}

#[test]
#[should_panic(expected = "#4)")] // InvalidThreshold
fn test_init_threshold_zero_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let token = make_token(&env).0;
    // Passing threshold=0 to the constructor should panic.
    env.register(
        EscrowContract,
        (
            approvers,
            0_u32,
            Address::generate(&env),
            token,
            milestones,
            env.ledger().sequence() + 1000,
        ),
    );
}

#[test]
#[should_panic(expected = "#4)")] // InvalidThreshold
fn test_init_threshold_exceeds_n_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let token = make_token(&env).0;
    env.register(
        EscrowContract,
        (
            approvers,
            3_u32, // threshold > N=2
            Address::generate(&env),
            token,
            milestones,
            env.ledger().sequence() + 1000,
        ),
    );
}

#[test]
#[should_panic(expected = "#5)")] // DuplicateApprover
fn test_init_duplicate_approver_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let dup = Address::generate(&env);
    let approvers = vec![&env, dup.clone(), dup];
    let token = make_token(&env).0;
    env.register(
        EscrowContract,
        (
            approvers,
            2_u32,
            Address::generate(&env),
            token,
            milestones,
            env.ledger().sequence() + 1000,
        ),
    );
}

#[test]
#[should_panic(expected = "#6)")] // InvalidMilestone
fn test_init_no_milestones_fails() {
    let env = setup_env();
    let milestones: soroban_sdk::Vec<i128> = vec![&env];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let token = make_token(&env).0;
    env.register(
        EscrowContract,
        (
            approvers,
            1_u32,
            Address::generate(&env),
            token,
            milestones,
            env.ledger().sequence() + 1000,
        ),
    );
}

#[test]
#[should_panic(expected = "#13)")] // AmountMustBePositive
fn test_init_zero_milestone_amount_fails() {
    let env = setup_env();
    let milestones = vec![&env, 0_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let token = make_token(&env).0;
    env.register(
        EscrowContract,
        (
            approvers,
            1_u32,
            Address::generate(&env),
            token,
            milestones,
            env.ledger().sequence() + 1000,
        ),
    );
}

#[test]
#[should_panic(expected = "#18)")] // InvalidExpiry (was #11 NotExpired)
fn test_init_expiry_not_in_future_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let token = make_token(&env).0;
    env.register(
        EscrowContract,
        (
            approvers,
            1_u32,
            Address::generate(&env),
            token,
            milestones,
            env.ledger().sequence(), // not strictly in the future
        ),
    );
}

// ---------------------------------------------------------------------------
// Funding cap tests (#1)
// ---------------------------------------------------------------------------

/// Funding exactly up to the milestone sum succeeds.
#[test]
fn test_fund_exact_cap_succeeds() {
    let env = setup_env();
    // Milestones: 1000 + 2000 = 3000 total cap.
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);

    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000); // exactly at cap — must succeed.
    assert_eq!(t.client.funded_total(), 3000);
}

/// Funding one stroop over the cap fails with FundingCapExceeded.
#[test]
#[should_panic(expected = "#16)")] // FundingCapExceeded
fn test_fund_over_cap_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);

    mint(&t.env, &t.token, &t.donor1, 3001);
    t.client.fund(&t.donor1, &3001); // one over the 3000 cap — must fail.
}

/// After all milestones are released the escrow is Completed; fund() is
/// rejected with AlreadyReleased rather than FundingCapExceeded because the
/// cap is inherently satisfied (no surplus can exist — released == funded).
#[test]
#[should_panic(expected = "#7)")] // AlreadyReleased (Completed state)
fn test_fund_after_all_milestones_released_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 1000);

    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);
    t.client.approve_milestone(&t.approver1, &0); // releases; state → Completed.
    assert_eq!(t.client.state(), State::Completed);

    // Try to fund again — Completed state rejects with AlreadyReleased.
    mint(&t.env, &t.token, &t.donor1, 1);
    t.client.fund(&t.donor1, &1);
}

// ---------------------------------------------------------------------------
// Fund tests
// ---------------------------------------------------------------------------

#[test]
fn test_fund_success() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);
    assert_eq!(t.client.funded_total(), 1000);
    assert_eq!(t.client.donor_contribution(&t.donor1), 1000);
}

#[test]
fn test_fund_multiple_donors() {
    let env = setup_env();
    let milestones = vec![&env, 3000_i128, 2000_i128];
    let t = setup(&env, 2, milestones, 1000);

    mint(&t.env, &t.token, &t.donor1, 3000);
    mint(&t.env, &t.token, &t.donor2, 2000);
    t.client.fund(&t.donor1, &3000);
    t.client.fund(&t.donor2, &2000);
    assert_eq!(t.client.funded_total(), 5000);
    assert_eq!(t.client.donor_contribution(&t.donor1), 3000);
    assert_eq!(t.client.donor_contribution(&t.donor2), 2000);
}

#[test]
#[should_panic(expected = "#13)")] // AmountMustBePositive
fn test_fund_zero_amount_fails() {
    let t = default_setup();
    t.client.fund(&t.donor1, &0);
}

#[test]
#[should_panic(expected = "#10)")] // Expired
fn test_fund_after_expiry_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 5);
    mint(&t.env, &t.token, &t.donor1, 500);
    env.ledger().set_sequence_number(t.expiry + 1);
    t.client.fund(&t.donor1, &500);
}

// ---------------------------------------------------------------------------
// Approval and threshold tests
// ---------------------------------------------------------------------------

#[test]
fn test_one_of_three_does_not_release() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);

    t.client.approve_milestone(&t.approver1, &0);
    assert_eq!(t.client.approvals(&0), 1);
    let (_, released) = t.client.milestone(&0);
    assert!(!released);
}

#[test]
fn test_two_of_three_releases() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 2, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);

    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver2, &0);
    assert_eq!(t.client.approvals(&0), 2);
    let (_, released) = t.client.milestone(&0);
    assert!(released);
    assert_eq!(t.client.released_total(), 1000);
}

#[test]
#[should_panic(expected = "#8)")] // AlreadyApproved
fn test_duplicate_approval_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver1, &0);
}

#[test]
#[should_panic(expected = "#3)")] // Unauthorized
fn test_non_approver_rejected() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);
    let stranger = Address::generate(&t.env);
    t.client.approve_milestone(&stranger, &0);
}

// ---------------------------------------------------------------------------
// Release blocked by insufficient funds, then via release_milestone
// ---------------------------------------------------------------------------

#[test]
fn test_release_blocked_then_succeeds_via_release_milestone() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 1000);

    // Fund 500 — below the 1000 milestone but within cap.
    mint(&t.env, &t.token, &t.donor1, 500);
    t.client.fund(&t.donor1, &500);

    // Approve — threshold reached but funds short; approval recorded, no release.
    t.client.approve_milestone(&t.approver1, &0);
    assert_eq!(t.client.approvals(&0), 1);
    let (_, released) = t.client.milestone(&0);
    assert!(!released);

    // Fund the remaining 500.
    mint(&t.env, &t.token, &t.donor2, 500);
    t.client.fund(&t.donor2, &500);

    // release_milestone should now succeed.
    t.client.release_milestone(&0);
    let (_, released) = t.client.milestone(&0);
    assert!(released);
    assert_eq!(t.client.released_total(), 1000);
}

// ---------------------------------------------------------------------------
// release_milestone rejection tests — #3: threshold uses ThresholdNotMet
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "#17)")] // ThresholdNotMet (was #9 InsufficientFunds)
fn test_release_milestone_threshold_not_met_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);
    // No approvals — threshold not met.
    t.client.release_milestone(&0);
}

#[test]
#[should_panic(expected = "#7)")] // AlreadyReleased
fn test_release_milestone_already_released_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver2, &0);
    t.client.release_milestone(&0); // already released
}

#[test]
#[should_panic(expected = "#14)")] // OutOfOrder
fn test_release_milestone_out_of_order_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);

    t.client.approve_milestone(&t.approver1, &1);
    t.client.release_milestone(&1); // out of order
}

#[test]
#[should_panic(expected = "#10)")] // Expired
fn test_release_milestone_after_expiry_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 5);
    // Fund 500 — short of milestone so it won't auto-release on approve.
    mint(&t.env, &t.token, &t.donor1, 500);
    t.client.fund(&t.donor1, &500);
    t.client.approve_milestone(&t.approver1, &0);
    // Milestone not released (insufficient funds).
    let (_, released) = t.client.milestone(&0);
    assert!(!released);
    // Advance past expiry.
    env.ledger().set_sequence_number(t.expiry + 1);
    // release_milestone must fail with Expired.
    t.client.release_milestone(&0);
}

// ---------------------------------------------------------------------------
// In-order release tests
// ---------------------------------------------------------------------------

#[test]
fn test_milestones_release_in_order() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);

    t.client.approve_milestone(&t.approver1, &0);
    let (_, released0) = t.client.milestone(&0);
    assert!(released0);
    assert_eq!(t.client.released_total(), 1000);

    t.client.approve_milestone(&t.approver1, &1);
    let (_, released1) = t.client.milestone(&1);
    assert!(released1);
    assert_eq!(t.client.released_total(), 3000);

    assert_eq!(t.client.state(), State::Completed);
}

// ---------------------------------------------------------------------------
// Double release impossibility
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "#7)")] // AlreadyReleased
fn test_double_release_impossible() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver2, &0);
    t.client.release_milestone(&0); // already released
}

// ---------------------------------------------------------------------------
// Refund tests
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "#11)")] // NotExpired
fn test_refund_before_expiry_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);
    t.client.refund(&t.donor1);
}

#[test]
fn test_refund_after_expiry_pro_rata() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 10);

    // donor1 → 3000, donor2 → 2000 (total 5000; cap is 3000).
    // Use milestones that sum to 5000 so cap allows this.
    let env2 = setup_env();
    let milestones2 = vec![&env2, 3000_i128, 2000_i128];
    let t2 = setup(&env2, 1, milestones2, 10);

    mint(&t2.env, &t2.token, &t2.donor1, 3000);
    mint(&t2.env, &t2.token, &t2.donor2, 2000);
    t2.client.fund(&t2.donor1, &3000);
    t2.client.fund(&t2.donor2, &2000);
    assert_eq!(t2.client.funded_total(), 5000);

    // Release milestone 0 (3000) → released_total = 3000.
    t2.client.approve_milestone(&t2.approver1, &0);
    assert_eq!(t2.client.released_total(), 3000);

    // Advance past expiry. State → Expired (milestone 1 still unreleased).
    env2.ledger().set_sequence_number(t2.expiry + 1);
    assert_eq!(t2.client.state(), State::Expired);

    // Unreleased = 5000 - 3000 = 2000.
    // donor1 refund = 3000 * 2000 / 5000 = 1200.
    // donor2 refund = 2000 * 2000 / 5000 = 800.
    let r1 = t2.client.refund(&t2.donor1);
    assert_eq!(r1, 1200);
    let r2 = t2.client.refund(&t2.donor2);
    assert_eq!(r2, 800);
    assert_eq!(r1 + r2, 2000);

    // Ignore t (unused); the test above uses t2.
    let _ = t;
}

#[test]
#[should_panic(expected = "#12)")] // NothingToRefund
fn test_double_refund_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 10);
    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);

    env.ledger().set_sequence_number(t.expiry + 1);
    t.client.refund(&t.donor1);
    t.client.refund(&t.donor1); // double refund
}

/// Refund after a partial release: some milestones released, then expiry.
/// Verifies that the pro-rata math correctly uses funded_total - released_total.
#[test]
fn test_refund_after_partial_release() {
    let env = setup_env();
    // 3 milestones of 1000 each; cap = 3000.
    let milestones = vec![&env, 1000_i128, 1000_i128, 1000_i128];
    let t = setup(&env, 1, milestones, 100);

    // Two donors: donor1 = 2000, donor2 = 1000 (total = 3000 = cap).
    mint(&t.env, &t.token, &t.donor1, 2000);
    mint(&t.env, &t.token, &t.donor2, 1000);
    t.client.fund(&t.donor1, &2000);
    t.client.fund(&t.donor2, &1000);
    assert_eq!(t.client.funded_total(), 3000);

    // Release milestone 0 → released_total = 1000.
    t.client.approve_milestone(&t.approver1, &0);
    assert_eq!(t.client.released_total(), 1000);

    // Milestones 1 and 2 not released. Advance past expiry.
    env.ledger().set_sequence_number(t.expiry + 1);
    assert_eq!(t.client.state(), State::Expired);

    // Unreleased = 3000 - 1000 = 2000.
    // donor1 refund = 2000 * 2000 / 3000 = 1333 (rounds down).
    // donor2 refund = 1000 * 2000 / 3000 = 666 (rounds down).
    // Total paid = 1333 + 666 = 1999 ≤ 2000. Dust (1 stroop) stays in contract.
    let r1 = t.client.refund(&t.donor1);
    let r2 = t.client.refund(&t.donor2);
    assert_eq!(r1, 1333);
    assert_eq!(r2, 666);
    // Total payout (released + refunds) must not exceed funded total.
    assert!(t.client.released_total() + r1 + r2 <= t.client.funded_total());
}

// ---------------------------------------------------------------------------
// Completed state
// ---------------------------------------------------------------------------

#[test]
fn test_completed_state_after_all_milestones() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 3000);
    t.client.fund(&t.donor1, &3000);

    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver1, &1);

    assert_eq!(t.client.state(), State::Completed);
    assert_eq!(t.client.released_total(), 3000);
}

// ---------------------------------------------------------------------------
// maintain_ttl tests (#2)
// ---------------------------------------------------------------------------

/// maintain_ttl on a fresh escrow (no persistent entries written yet)
/// must not panic — it should skip extend_ttl for non-existent entries.
#[test]
fn test_maintain_ttl_fresh_escrow() {
    let t = default_setup();
    // No activity — no persistent milestone or donor entries exist yet.
    // This must not panic even though the entries don't exist.
    t.client.maintain_ttl(&0);
}

/// maintain_ttl after activity (persistent entries do exist) must succeed.
#[test]
fn test_maintain_ttl_after_activity() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);

    // Create persistent entries by funding, approving, and releasing.
    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);
    t.client.approve_milestone(&t.approver1, &0);
    // milestone 0 released; milestone 1 has an approval count entry.

    // maintain_ttl must not panic.
    t.client.maintain_ttl(&0);
    // Can also start from index 1 — must not panic.
    t.client.maintain_ttl(&1);
}

/// maintain_donor_ttl for a donor with and without entries must not panic.
#[test]
fn test_maintain_donor_ttl() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 100);

    // No entries yet — must not panic.
    t.client.maintain_donor_ttl(&t.donor1);

    // Create contribution entry.
    mint(&t.env, &t.token, &t.donor1, 1000);
    t.client.fund(&t.donor1, &1000);

    // Contribution entry exists now — must not panic.
    t.client.maintain_donor_ttl(&t.donor1);

    // Create refunded entry.
    env.ledger().set_sequence_number(t.expiry + 1);
    t.client.refund(&t.donor1);

    // Both entries exist — must not panic.
    t.client.maintain_donor_ttl(&t.donor1);
}

// ---------------------------------------------------------------------------
// Invariant test: genuinely randomized with seeded PRNG, multiple donors,
// partial releases (#8)
// ---------------------------------------------------------------------------

/// Simple seeded linear-congruential PRNG (Knuth MMIX).
/// Returns the next value and the updated state.
fn lcg_next(state: u64) -> (u64, u64) {
    let next = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (next, next)
}

/// Return a value in [min, max] given a PRNG state.
fn rand_range(state: u64, min: i128, max: i128) -> (i128, u64) {
    let (val, next) = lcg_next(state);
    let range = (max - min + 1) as u64;
    (min + (val % range) as i128, next)
}

#[test]
fn test_invariant_payout_never_exceeds_funded_randomized() {
    // Run the invariant across several seeds to get deterministic but varied coverage.
    let seeds: [u64; 6] = [
        0xDEAD_BEEF,
        0x1234_5678_9ABC_DEF0,
        0xCAFE_BABE,
        0x0000_0001,
        0xFFFF_FFFF_FFFF_FFFE,
        0xABCD_EF01_2345_6789,
    ];

    for &seed in &seeds {
        let env = setup_env();
        let mut rng = seed;

        // Generate 2–4 milestone amounts (10–500 each).
        let (n_milestones_raw, r) = lcg_next(rng);
        rng = r;
        let n_milestones = 2 + (n_milestones_raw % 3) as u32; // 2, 3, or 4

        let mut milestone_amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(&env);
        let mut milestone_sum: i128 = 0;
        for _ in 0..n_milestones {
            let (amt, r) = rand_range(rng, 10, 500);
            rng = r;
            milestone_amounts.push_back(amt);
            milestone_sum += amt;
        }

        let t = setup(&env, 1, milestone_amounts.clone(), 1000);

        // 2–4 donors each funding a random share of the cap.
        let (n_donors_raw, r) = lcg_next(rng);
        rng = r;
        let n_donors = 2 + (n_donors_raw % 3) as u32; // 2, 3, or 4

        let mut donors_who_funded: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        let mut total_funded: i128 = 0;
        let mut remaining_cap = milestone_sum;

        for d in 0..n_donors {
            let donor = Address::generate(&env);

            // Last donor funds exactly what remains so funded_total == milestone_sum.
            let fund_amount = if d == n_donors - 1 {
                remaining_cap
            } else {
                let max_for_this = remaining_cap / (n_donors - d) as i128;
                let max_for_this = max_for_this.max(1);
                let (amt, r) = rand_range(rng, 1, max_for_this);
                rng = r;
                amt
            };

            if fund_amount <= 0 {
                continue;
            }
            mint(&t.env, &t.token, &donor, fund_amount);
            t.client.fund(&donor, &fund_amount);
            total_funded += fund_amount;
            remaining_cap -= fund_amount;
            donors_who_funded.push_back(donor);
        }

        assert_eq!(t.client.funded_total(), total_funded);

        // Partially release: release a random subset of milestones in order.
        let (n_release_raw, r) = lcg_next(rng);
        rng = r;
        // Release between 0 and n_milestones-1 milestones (leave at least one unreleased
        // so the state is Expired rather than Completed after the ledger advances).
        let n_to_release = (n_release_raw % n_milestones as u64) as u32;

        for i in 0..n_to_release {
            t.client.approve_milestone(&t.approver1, &i);
            let (_, released) = t.client.milestone(&i);
            assert!(released, "milestone {i} should be released");
        }

        let released_total = t.client.released_total();
        assert!(
            released_total <= total_funded,
            "seed {seed}: released_total ({released_total}) > funded ({total_funded})"
        );

        // Advance past expiry.
        env.ledger().set_sequence_number(t.expiry + 1);
        assert_eq!(
            t.client.state(),
            State::Expired,
            "seed {seed}: expected Expired state"
        );

        // Every donor who funded refunds (skip donors whose pro-rata share
        // rounds to 0 — the contract rejects them with NothingToRefund).
        let mut total_refunded: i128 = 0;
        let unreleased = total_funded - released_total;
        for donor in donors_who_funded.iter() {
            let contribution = t.client.donor_contribution(&donor);
            let expected = contribution * unreleased / total_funded;
            if expected <= 0 {
                continue;
            }
            let r = t.client.refund(&donor);
            total_refunded += r;
        }

        // Invariant: released + refunded ≤ funded.
        let total_paid = released_total + total_refunded;
        assert!(
            total_paid <= total_funded,
            "seed {seed}: total_paid ({total_paid}) > funded ({total_funded})"
        );

        let _ = rng; // suppress unused warning
    }
}
