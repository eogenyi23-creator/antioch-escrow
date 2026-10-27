/**
 * Amount helpers for Stellar token amounts.
 *
 * All on-chain values are `i128` integers in the token's smallest unit.
 * For USDC (7 decimals), 1 USDC = 10_000_000 stroops.
 *
 * These helpers operate entirely in string/bigint arithmetic and never use
 * floating-point numbers, matching the contract's strict integer-money policy.
 */

/** Number of decimal places for Stellar tokens (USDC and native XLM both use 7). */
export const DECIMALS = 7;

const SCALE = BigInt(10) ** BigInt(DECIMALS); // 10_000_000n

/**
 * Convert a decimal string such as "25.0000000" or "25" to the integer number
 * of stroops (smallest token units).
 *
 * Rules:
 * - At most 7 decimal places are allowed. More throws.
 * - The input must be a valid non-negative decimal string. Anything else throws.
 * - No exponential notation. No leading/trailing whitespace.
 *
 * @throws {Error} if the input is not a valid, non-negative decimal with at most 7 places.
 *
 * @example
 * toStroops("1.0000000") === 10_000_000n
 * toStroops("0.0000001") === 1n           // one stroop
 * toStroops("25")        === 250_000_000n
 */
export function toStroops(decimal: string): bigint {
  if (typeof decimal !== "string") {
    throw new Error(`toStroops: expected a string, got ${typeof decimal}`);
  }

  // Reject whitespace, exponential notation, and empty strings up-front.
  if (decimal.trim() !== decimal || decimal === "" || /[eE]/.test(decimal)) {
    throw new Error(`toStroops: invalid decimal string: "${decimal}"`);
  }

  const parts = decimal.split(".");
  if (parts.length > 2) {
    throw new Error(`toStroops: too many decimal points in "${decimal}"`);
  }

  const [integerPart, fractionalPart = ""] = parts;

  // Validate each part contains only digits (allow empty integerPart like ".5"? No — require at least "0").
  if (!/^\d+$/.test(integerPart)) {
    throw new Error(`toStroops: invalid integer part "${integerPart}" in "${decimal}"`);
  }
  if (fractionalPart !== "" && !/^\d+$/.test(fractionalPart)) {
    throw new Error(`toStroops: invalid fractional part "${fractionalPart}" in "${decimal}"`);
  }

  if (fractionalPart.length > DECIMALS) {
    throw new Error(
      `toStroops: fractional part has ${fractionalPart.length} digits; maximum is ${DECIMALS}. ` +
        `Input: "${decimal}"`
    );
  }

  // Pad the fractional part to exactly DECIMALS digits by appending zeros.
  const paddedFractional = fractionalPart.padEnd(DECIMALS, "0");

  const intBig = BigInt(integerPart) * SCALE;
  const fracBig = BigInt(paddedFractional);

  return intBig + fracBig;
}

/**
 * Convert a stroop count (integer number of smallest token units) to a decimal
 * string with exactly 7 decimal places.
 *
 * @param stroops The raw i128 value as a bigint or number. Must be >= 0.
 * @returns A string like "25.0000000".
 *
 * @throws {Error} if stroops is negative.
 *
 * @example
 * fromStroops(10_000_000n) === "1.0000000"
 * fromStroops(1n)          === "0.0000001"
 * fromStroops(0n)          === "0.0000000"
 */
export function fromStroops(stroops: bigint | number): string {
  const value = typeof stroops === "number" ? BigInt(stroops) : stroops;

  if (value < 0n) {
    throw new Error(`fromStroops: value must be >= 0, got ${value}`);
  }

  const integerPart = value / SCALE;
  const fractionalPart = value % SCALE;

  // Pad the fractional part to exactly DECIMALS digits.
  const fractionalStr = fractionalPart.toString().padStart(DECIMALS, "0");

  return `${integerPart}.${fractionalStr}`;
}
