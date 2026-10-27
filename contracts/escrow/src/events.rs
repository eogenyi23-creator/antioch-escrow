use soroban_sdk::{contractevent, Address, Env};

// ---------------------------------------------------------------------------
// Event types — each uses #[contractevent] so the event is included in the
// contract interface specification and tooling can understand it.
//
// The struct name is automatically lowercased as the first static topic
// (e.g. `EscrowInit` → "escrow_init").  Fields marked `#[topic]` become
// additional dynamic topics; the rest become the event data map.
// ---------------------------------------------------------------------------

/// Emitted once when the escrow is initialised.
/// Topics: ("escrow_init",). Data: { beneficiary, token, threshold, milestone_count }.
#[contractevent]
pub struct EscrowInit {
    pub beneficiary: Address,
    pub token: Address,
    pub threshold: u32,
    pub milestone_count: u32,
}

/// Emitted each time a donor funds the escrow.
/// Topics: ("escrow_fund", donor). Data: { amount }.
#[contractevent]
pub struct EscrowFund {
    #[topic]
    pub donor: Address,
    pub amount: i128,
}

/// Emitted each time an approver records an approval.
/// Topics: ("escrow_approve", approver, index). Data: {} (no extra data).
#[contractevent]
pub struct EscrowApprove {
    #[topic]
    pub approver: Address,
    #[topic]
    pub index: u32,
}

/// Emitted when a milestone is released to the beneficiary.
/// Topics: ("escrow_release", index). Data: { amount, beneficiary }.
#[contractevent]
pub struct EscrowRelease {
    #[topic]
    pub index: u32,
    pub amount: i128,
    pub beneficiary: Address,
}

/// Emitted when a donor receives a pro-rata refund after expiry.
/// Topics: ("escrow_refund", donor). Data: { amount }.
#[contractevent]
pub struct EscrowRefund {
    #[topic]
    pub donor: Address,
    pub amount: i128,
}

// ---------------------------------------------------------------------------
// Publish helpers
// ---------------------------------------------------------------------------

/// Emit the init event.
pub fn init(
    env: &Env,
    beneficiary: &Address,
    token: &Address,
    threshold: u32,
    milestone_count: u32,
) {
    EscrowInit {
        beneficiary: beneficiary.clone(),
        token: token.clone(),
        threshold,
        milestone_count,
    }
    .publish(env);
}

/// Emit the fund event.
pub fn fund(env: &Env, donor: &Address, amount: i128) {
    EscrowFund {
        donor: donor.clone(),
        amount,
    }
    .publish(env);
}

/// Emit the approve event.
pub fn approve(env: &Env, approver: &Address, index: u32) {
    EscrowApprove {
        approver: approver.clone(),
        index,
    }
    .publish(env);
}

/// Emit the release event.
pub fn release(env: &Env, index: u32, amount: i128, beneficiary: &Address) {
    EscrowRelease {
        index,
        amount,
        beneficiary: beneficiary.clone(),
    }
    .publish(env);
}

/// Emit the refund event.
pub fn refund(env: &Env, donor: &Address, amount: i128) {
    EscrowRefund {
        donor: donor.clone(),
        amount,
    }
    .publish(env);
}
