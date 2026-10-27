use soroban_sdk::contracterror;

/// Every failure path in the contract has a typed error.
/// No `panic!` or `unwrap()` is used in contract code.
#[contracterror]
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The contract has not been initialized yet.
    NotInitialized = 1,
    /// `init` was already called; it can only run once.
    AlreadyInitialized = 2,
    /// The caller is not authorized for this action.
    Unauthorized = 3,
    /// The threshold is outside the valid range (1..=N).
    InvalidThreshold = 4,
    /// An approver address appears more than once in the set.
    DuplicateApprover = 5,
    /// The milestone index is out of bounds.
    InvalidMilestone = 6,
    /// The milestone has already been released.
    AlreadyReleased = 7,
    /// The approver has already approved this milestone.
    AlreadyApproved = 8,
    /// The contract holds insufficient unreleased funds.
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
}
