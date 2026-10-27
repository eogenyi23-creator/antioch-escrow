use soroban_sdk::contracterror;

/// Every failure path in the contract has a typed error.
/// No `panic!` or `unwrap()` is used in contract code.
#[contracterror]
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The contract has not been initialized yet.
    NotInitialized = 1,
    /// `init` (or `__constructor`) was already called; it can only run once.
    AlreadyInitialized = 2,
    /// The caller is not authorized for this action.
    Unauthorized = 3,
    /// The threshold is outside the valid range (1..=N).
    InvalidThreshold = 4,
    /// An approver address appears more than once in the set.
    DuplicateApprover = 5,
    /// The milestone index is out of bounds, or the milestones list is empty.
    InvalidMilestone = 6,
    /// The milestone has already been released.
    AlreadyReleased = 7,
    /// The approver has already approved this milestone.
    AlreadyApproved = 8,
    /// The contract holds insufficient unreleased funds to cover the milestone.
    InsufficientFunds = 9,
    /// The escrow has expired; only `refund` works now.
    Expired = 10,
    /// The escrow has not expired yet; `refund` is not available.
    NotExpired = 11,
    /// The donor has nothing to refund (zero contribution or already refunded).
    NothingToRefund = 12,
    /// An amount must be strictly positive.
    AmountMustBePositive = 13,
    /// The milestone cannot be released out of order (n-1 must be released first).
    OutOfOrder = 14,
    /// An arithmetic operation would overflow `i128`.
    Overflow = 15,
    /// Funding this amount would exceed the total of all milestone amounts.
    ///
    /// # Errors
    /// Returned by `fund` when `funded_total + amount > sum(milestones)`.
    FundingCapExceeded = 16,
    /// The milestone's approval threshold has not yet been reached.
    ///
    /// # Errors
    /// Returned by `release_milestone` when fewer than `threshold` approvals
    /// have been recorded for the given milestone index.
    ThresholdNotMet = 17,
    /// The `expiry_ledger` supplied to the constructor is not strictly in the
    /// future (i.e. it is ≤ the current ledger sequence at construction time).
    ///
    /// # Errors
    /// Returned by `__constructor` when `expiry_ledger <= env.ledger().sequence()`.
    InvalidExpiry = 18,
}
