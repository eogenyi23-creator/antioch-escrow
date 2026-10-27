/**
 * Error decoding for the Antioch Escrow contract.
 *
 * Soroban contracts raise typed errors via `panic_with_error`. On-chain these
 * surface as a `ContractError` whose numeric code corresponds to the `Error`
 * enum in the Rust contract. This module maps those numeric codes back to
 * human-readable error names and messages.
 *
 * The full enum (from `contracts/escrow/src/error.rs`):
 *
 * ```rust
 * #[contracterror]
 * #[repr(u32)]
 * pub enum Error {
 *     NotInitialized = 1,
 *     AlreadyInitialized = 2,
 *     Unauthorized = 3,
 *     InvalidThreshold = 4,
 *     DuplicateApprover = 5,
 *     InvalidMilestone = 6,
 *     AlreadyReleased = 7,
 *     AlreadyApproved = 8,
 *     InsufficientFunds = 9,
 *     Expired = 10,
 *     NotExpired = 11,
 *     NothingToRefund = 12,
 *     AmountMustBePositive = 13,
 *     OutOfOrder = 14,
 *     Overflow = 15,
 *     FundingCapExceeded = 16,
 *     ThresholdNotMet = 17,
 *     InvalidExpiry = 18,
 * }
 * ```
 */

/** The numeric error code returned by the Soroban runtime. */
export type ErrorCode = number;

/** A decoded contract error with its symbolic name and a human-readable message. */
export interface DecodedError {
  /** The numeric error code from the contract. */
  code: ErrorCode;
  /** The Rust enum variant name, e.g. `"FundingCapExceeded"`. */
  name: string;
  /** A human-readable description of what went wrong. */
  message: string;
}

const ERROR_MAP: Record<ErrorCode, { name: string; message: string }> = {
  1: {
    name: "NotInitialized",
    message: "The contract has not been initialized yet.",
  },
  2: {
    name: "AlreadyInitialized",
    message: "The contract has already been initialized; init can only run once.",
  },
  3: {
    name: "Unauthorized",
    message: "The caller is not authorized for this action.",
  },
  4: {
    name: "InvalidThreshold",
    message: "The threshold is outside the valid range (1..=N).",
  },
  5: {
    name: "DuplicateApprover",
    message: "An approver address appears more than once in the set.",
  },
  6: {
    name: "InvalidMilestone",
    message: "The milestone index is out of bounds, or the milestones list is empty.",
  },
  7: {
    name: "AlreadyReleased",
    message: "The milestone has already been released.",
  },
  8: {
    name: "AlreadyApproved",
    message: "The approver has already approved this milestone.",
  },
  9: {
    name: "InsufficientFunds",
    message: "The contract holds insufficient unreleased funds to cover the milestone.",
  },
  10: {
    name: "Expired",
    message: "The escrow has expired; only refund works now.",
  },
  11: {
    name: "NotExpired",
    message: "The escrow has not expired yet; refund is not available.",
  },
  12: {
    name: "NothingToRefund",
    message: "The donor has nothing to refund (zero contribution or already refunded).",
  },
  13: {
    name: "AmountMustBePositive",
    message: "An amount must be strictly positive.",
  },
  14: {
    name: "OutOfOrder",
    message: "The milestone cannot be released out of order (n-1 must be released first).",
  },
  15: {
    name: "Overflow",
    message: "An arithmetic operation would overflow i128.",
  },
  16: {
    name: "FundingCapExceeded",
    message: "Funding this amount would exceed the total of all milestone amounts.",
  },
  17: {
    name: "ThresholdNotMet",
    message: "The milestone's approval threshold has not yet been reached.",
  },
  18: {
    name: "InvalidExpiry",
    message: "The expiry_ledger supplied to the constructor is not strictly in the future.",
  },
};

/**
 * Decode a numeric Soroban contract error code into a human-readable error.
 *
 * @param code The numeric error code (e.g. 16 for FundingCapExceeded).
 * @returns A `DecodedError` with the symbolic name and message.
 * @throws {Error} if the code is not a known contract error.
 *
 * @example
 * ```ts
 * const decoded = decodeError(16);
 * // { code: 16, name: "FundingCapExceeded", message: "Funding this amount would exceed ..." }
 * ```
 */
export function decodeError(code: ErrorCode): DecodedError {
  const entry = ERROR_MAP[code];
  if (!entry) {
    throw new Error(`Unknown contract error code: ${code}`);
  }
  return { code, ...entry };
}

/**
 * Decode an error from a Soroban simulation or transaction result.
 *
 * Accepts either a numeric error code directly, or an object that contains an
 * `error` property (as returned by `simulateTransaction` failures), or a string
 * that can be parsed to a number.
 *
 * @param error The raw error from the Soroban RPC call.
 * @returns A `DecodedError`, or `null` if the error cannot be mapped to a
 *          known contract error code.
 *
 * @example
 * ```ts
 * try {
 *   await escrow.readState();
 * } catch (e) {
 *   const decoded = decodeError(e);
 *   if (decoded) {
 *     console.error(`${decoded.name}: ${decoded.message}`);
 *   }
 * }
 * ```
 */
export function decodeErrorFromResult(error: unknown): DecodedError | null {
  // Handle numeric codes directly.
  if (typeof error === "number") {
    try {
      return decodeError(error);
    } catch {
      return null;
    }
  }

  // Handle string codes — extract the first number found.
  if (typeof error === "string") {
    const match = error.match(/error.*?(\d+)/i);
    if (match) {
      try {
        return decodeError(parseInt(match[1], 10));
      } catch {
        return null;
      }
    }
  }

  // Handle error objects with a numeric `code` property.
  if (error !== null && typeof error === "object") {
    const obj = error as Record<string, unknown>;
    if (typeof obj.code === "number") {
      try {
        return decodeError(obj.code);
      } catch {
        return null;
      }
    }
    // Handle Soroban simulation errors that nest the code.
    if (obj.error !== undefined) {
      return decodeErrorFromResult(obj.error);
    }
  }

  return null;
}
