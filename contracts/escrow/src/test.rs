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
    _contract_id: Address,
    token: Address,
    _token_admin: Address,
    beneficiary: Address,
    approver1: Address,
    approver2: Address,
    approver3: Address,
    donor1: Address,
    donor2: Address,
    milestones: soroban_sdk::Vec<i128>,
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

fn setup(
    env: &Env,
    threshold: u32,
    milestones: soroban_sdk::Vec<i128>,
    expiry_offset: u32,
) -> TestSetup {
    let contract_id = env.register(EscrowContract, ());
    let (token, token_admin) = make_token(env);

    let beneficiary = Address::generate(env);
    let approver1 = Address::generate(env);
    let approver2 = Address::generate(env);
    let approver3 = Address::generate(env);
    let donor1 = Address::generate(env);
    let donor2 = Address::generate(env);

    let approvers = vec![env, approver1.clone(), approver2.clone(), approver3.clone()];

    let expiry = env.ledger().sequence() + expiry_offset;

    let client = EscrowContractClient::new(env, &contract_id);
    client.init(
        &approvers,
        &threshold,
        &beneficiary,
        &token,
        &milestones,
        &expiry,
    );

    TestSetup {
        env: env.clone(),
        client,
        _contract_id: contract_id,
        token,
        _token_admin: token_admin,
        beneficiary,
        approver1,
        approver2,
        approver3,
        donor1,
        donor2,
        milestones,
        expiry,
    }
}

fn default_setup() -> TestSetup {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    setup(&env, 2, milestones, 1000)
}

/// Mint tokens to an address using the mock SAC token.
/// Uses StellarAssetClient which includes admin functions like mint.
fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    use soroban_sdk::token::StellarAssetClient;
    let sac = StellarAssetClient::new(env, token);
    // mock_all_auths is already set on the env, so admin auth is mocked.
    sac.mint(to, &amount);
}

// ---------------------------------------------------------------------------
// Init tests
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

#[test]
#[should_panic(expected = "#2)")] // AlreadyInitialized
fn test_init_double_init_fails() {
    let t = default_setup();
    let approvers = vec![
        &t.env,
        t.approver1.clone(),
        t.approver2.clone(),
        t.approver3.clone(),
    ];
    t.client.init(
        &approvers,
        &2,
        &t.beneficiary,
        &t.token,
        &t.milestones,
        &t.expiry,
    );
}

#[test]
#[should_panic(expected = "#4)")] // InvalidThreshold
fn test_init_threshold_zero_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.init(
        &approvers,
        &0,
        &Address::generate(&env),
        &Address::generate(&env),
        &milestones,
        &(env.ledger().sequence() + 1000),
    );
}

#[test]
#[should_panic(expected = "#4)")] // InvalidThreshold
fn test_init_threshold_exceeds_n_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.init(
        &approvers,
        &3, // threshold > N=2
        &Address::generate(&env),
        &Address::generate(&env),
        &milestones,
        &(env.ledger().sequence() + 1000),
    );
}

#[test]
#[should_panic(expected = "#5)")] // DuplicateApprover
fn test_init_duplicate_approver_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let dup = Address::generate(&env);
    let approvers = vec![&env, dup.clone(), dup];
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.init(
        &approvers,
        &2,
        &Address::generate(&env),
        &Address::generate(&env),
        &milestones,
        &(env.ledger().sequence() + 1000),
    );
}

#[test]
#[should_panic(expected = "#6)")] // InvalidMilestone
fn test_init_no_milestones_fails() {
    let env = setup_env();
    let milestones = vec![&env];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.init(
        &approvers,
        &1,
        &Address::generate(&env),
        &Address::generate(&env),
        &milestones,
        &(env.ledger().sequence() + 1000),
    );
}

#[test]
#[should_panic(expected = "#13)")] // AmountMustBePositive
fn test_init_zero_milestone_amount_fails() {
    let env = setup_env();
    let milestones = vec![&env, 0_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.init(
        &approvers,
        &1,
        &Address::generate(&env),
        &Address::generate(&env),
        &milestones,
        &(env.ledger().sequence() + 1000),
    );
}

#[test]
#[should_panic(expected = "#11)")] // NotExpired
fn test_init_expiry_not_in_future_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let approvers = vec![&env, Address::generate(&env), Address::generate(&env)];
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    // expiry = current ledger (not strictly in the future)
    client.init(
        &approvers,
        &1,
        &Address::generate(&env),
        &Address::generate(&env),
        &milestones,
        &env.ledger().sequence(),
    );
}

// ---------------------------------------------------------------------------
// Fund tests
// ---------------------------------------------------------------------------

#[test]
fn test_fund_success() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    assert_eq!(t.client.funded_total(), 5000);
    assert_eq!(t.client.donor_contribution(&t.donor1), 5000);
}

#[test]
fn test_fund_multiple_donors() {
    let t = default_setup();
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
    mint(&t.env, &t.token, &t.donor1, 5000);
    // Advance ledger past expiry.
    env.ledger().set_sequence_number(t.expiry + 1);
    t.client.fund(&t.donor1, &1000);
}

// ---------------------------------------------------------------------------
// Approval and threshold tests
// ---------------------------------------------------------------------------

#[test]
fn test_one_of_three_does_not_release() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);

    // 1 of 3 approvals — not enough (threshold = 2).
    t.client.approve_milestone(&t.approver1, &0);
    assert_eq!(t.client.approvals(&0), 1);
    // Milestone should NOT be released.
    let (_, released) = t.client.milestone(&0);
    assert!(!released);
}

#[test]
fn test_two_of_three_releases() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);

    // 2 of 3 approvals — enough (threshold = 2).
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver2, &0);
    assert_eq!(t.client.approvals(&0), 2);
    // Milestone should be released.
    let (_, released) = t.client.milestone(&0);
    assert!(released);
    assert_eq!(t.client.released_total(), 1000);
}

#[test]
#[should_panic(expected = "#8)")] // AlreadyApproved
fn test_duplicate_approval_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver1, &0); // duplicate
}

#[test]
#[should_panic(expected = "#3)")] // Unauthorized
fn test_non_approver_rejected() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
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

    // Don't fund enough yet.
    mint(&t.env, &t.token, &t.donor1, 500);
    t.client.fund(&t.donor1, &500);

    // 1 of 1 approval (threshold = 1) but funds are short (500 < 1000).
    t.client.approve_milestone(&t.approver1, &0);
    assert_eq!(t.client.approvals(&0), 1);
    // Milestone should NOT be released.
    let (_, released) = t.client.milestone(&0);
    assert!(!released);

    // Fund more.
    mint(&t.env, &t.token, &t.donor2, 1000);
    t.client.fund(&t.donor2, &1000);

    // Now release_milestone should succeed.
    t.client.release_milestone(&0);
    let (_, released) = t.client.milestone(&0);
    assert!(released);
    assert_eq!(t.client.released_total(), 1000);
}

// ---------------------------------------------------------------------------
// release_milestone rejection tests
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "#9)")] // InsufficientFunds
fn test_release_milestone_threshold_not_met_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    // No approvals yet — threshold not met.
    t.client.release_milestone(&0);
}

#[test]
#[should_panic(expected = "#7)")] // AlreadyReleased
fn test_release_milestone_already_released_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver2, &0);
    // Milestone 0 is released automatically.
    t.client.release_milestone(&0); // already released
}

#[test]
#[should_panic(expected = "#14)")] // OutOfOrder
fn test_release_milestone_out_of_order_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);

    // Approve milestone 1 but not milestone 0.
    t.client.approve_milestone(&t.approver1, &1);
    // Try to release milestone 1 before milestone 0 — should fail.
    t.client.release_milestone(&1);
}

#[test]
#[should_panic(expected = "#10)")] // Expired
fn test_release_milestone_after_expiry_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 5);
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    t.client.approve_milestone(&t.approver1, &0);

    // Advance ledger past expiry (but milestone is NOT released yet since
    // funds were sufficient, so it would have been auto-released).
    // Actually with threshold=1 and enough funds, it auto-releases.
    // So we need a scenario where it didn't auto-release.
    // Let's use insufficient funds + expiry.
    // Redo: fund 500, approve, then expire.
    let env2 = setup_env();
    let milestones2 = vec![&env2, 1000_i128];
    let t2 = setup(&env2, 1, milestones2, 5);
    mint(&t2.env, &t2.token, &t2.donor1, 500);
    t2.client.fund(&t2.donor1, &500);
    t2.client.approve_milestone(&t2.approver1, &0);
    // Milestone not released (insufficient funds).
    let (_, released) = t2.client.milestone(&0);
    assert!(!released);
    // Advance past expiry.
    env2.ledger().set_sequence_number(t2.expiry + 1);
    // Now release_milestone should fail with Expired.
    t2.client.release_milestone(&0);
}

// ---------------------------------------------------------------------------
// In-order release tests
// ---------------------------------------------------------------------------

#[test]
fn test_milestones_release_in_order() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);

    // Release milestone 0.
    t.client.approve_milestone(&t.approver1, &0);
    let (_, released0) = t.client.milestone(&0);
    assert!(released0);
    assert_eq!(t.client.released_total(), 1000);

    // Now release milestone 1.
    t.client.approve_milestone(&t.approver1, &1);
    let (_, released1) = t.client.milestone(&1);
    assert!(released1);
    assert_eq!(t.client.released_total(), 3000);

    // State should be Completed.
    assert_eq!(t.client.state(), State::Completed);
}

// ---------------------------------------------------------------------------
// Double release impossibility
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "#7)")] // AlreadyReleased
fn test_double_release_impossible() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver2, &0);
    // Milestone 0 auto-released. Try to release again.
    t.client.release_milestone(&0);
}

// ---------------------------------------------------------------------------
// Refund tests
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "#11)")] // NotExpired
fn test_refund_before_expiry_fails() {
    let t = default_setup();
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);
    t.client.refund(&t.donor1);
}

#[test]
fn test_refund_after_expiry_pro_rata() {
    let env = setup_env();
    // Two milestones so that releasing only milestone 0 leaves milestone 1
    // unreleased — the state is Expired (not Completed) after the ledger passes.
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 10);

    // Two donors fund: donor1 gives 3000, donor2 gives 2000 (total 5000).
    mint(&t.env, &t.token, &t.donor1, 3000);
    mint(&t.env, &t.token, &t.donor2, 2000);
    t.client.fund(&t.donor1, &3000);
    t.client.fund(&t.donor2, &2000);
    assert_eq!(t.client.funded_total(), 5000);

    // Release milestone 0 (amount 1000) — released_total = 1000.
    t.client.approve_milestone(&t.approver1, &0);
    assert_eq!(t.client.released_total(), 1000);

    // Advance past expiry. State is Expired (milestone 1 is still unreleased).
    env.ledger().set_sequence_number(t.expiry + 1);
    assert_eq!(t.client.state(), State::Expired);

    // Unreleased = 5000 - 1000 = 4000.
    // donor1 refund = 3000 * 4000 / 5000 = 2400.
    // donor2 refund = 2000 * 4000 / 5000 = 1600.
    let r1 = t.client.refund(&t.donor1);
    assert_eq!(r1, 2400);
    let r2 = t.client.refund(&t.donor2);
    assert_eq!(r2, 1600);

    // Total refunded = 4000 = unreleased amount. No dust leak.
    assert_eq!(r1 + r2, 4000);
}

#[test]
#[should_panic(expected = "#12)")] // NothingToRefund
fn test_double_refund_fails() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128];
    let t = setup(&env, 1, milestones, 10);
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);

    env.ledger().set_sequence_number(t.expiry + 1);
    t.client.refund(&t.donor1);
    t.client.refund(&t.donor1); // double refund
}

// ---------------------------------------------------------------------------
// Completed state
// ---------------------------------------------------------------------------

#[test]
fn test_completed_state_after_all_milestones() {
    let env = setup_env();
    let milestones = vec![&env, 1000_i128, 2000_i128];
    let t = setup(&env, 1, milestones, 1000);
    mint(&t.env, &t.token, &t.donor1, 5000);
    t.client.fund(&t.donor1, &5000);

    t.client.approve_milestone(&t.approver1, &0);
    t.client.approve_milestone(&t.approver1, &1);

    assert_eq!(t.client.state(), State::Completed);
    assert_eq!(t.client.released_total(), 3000);
}

// ---------------------------------------------------------------------------
// Invariant test: total paid out never exceeds total funded
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_payout_never_exceeds_funded() {
    // Test with several random-ish scenarios.
    // Use two milestones so that after releasing only the first,
    // the state is Expired (not Completed) when we advance the ledger.
    let amounts = [100_i128, 500, 999, 1000, 1234, 50000, 999999];

    for &milestone_amount in &amounts {
        let env = setup_env();
        // Second milestone is large and never released, keeping state Expired.
        let milestones = vec![&env, milestone_amount, 999_999_999_i128];
        let t = setup(&env, 1, milestones, 1000);

        // Fund an amount that covers the first milestone but not the second.
        let fund_amount = milestone_amount + 777;
        mint(&t.env, &t.token, &t.donor1, fund_amount);
        t.client.fund(&t.donor1, &fund_amount);

        // Release the first milestone.
        t.client.approve_milestone(&t.approver1, &0);

        assert!(t.client.released_total() <= t.client.funded_total());

        // Advance past expiry and refund.
        env.ledger().set_sequence_number(t.expiry + 1);
        assert_eq!(t.client.state(), State::Expired);
        let refund = t.client.refund(&t.donor1);

        // Total paid out = released + refund <= funded.
        let total_paid = t.client.released_total() + refund;
        assert!(
            total_paid <= t.client.funded_total(),
            "total_paid ({}) > funded ({}) for milestone_amount {}",
            total_paid,
            t.client.funded_total(),
            milestone_amount
        );
    }
}
