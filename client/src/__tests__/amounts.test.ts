import { DECIMALS, fromStroops, toStroops } from "../amounts";

describe("DECIMALS", () => {
  it("is 7", () => {
    expect(DECIMALS).toBe(7);
  });
});

describe("toStroops", () => {
  // Correct conversions
  it("converts 1.0000000 to 10_000_000n", () => {
    expect(toStroops("1.0000000")).toBe(10_000_000n);
  });

  it("converts 0.0000001 (one stroop) to 1n", () => {
    expect(toStroops("0.0000001")).toBe(1n);
  });

  it("converts 0.0000000 to 0n", () => {
    expect(toStroops("0.0000000")).toBe(0n);
  });

  it("converts 25.0000000 to 250_000_000n", () => {
    expect(toStroops("25.0000000")).toBe(250_000_000n);
  });

  it("converts integer string (no decimal) correctly", () => {
    expect(toStroops("25")).toBe(250_000_000n);
    expect(toStroops("0")).toBe(0n);
  });

  it("pads short fractional parts with trailing zeros", () => {
    // "1.5" == "1.5000000" == 15_000_000
    expect(toStroops("1.5")).toBe(15_000_000n);
    expect(toStroops("1.50")).toBe(15_000_000n);
    expect(toStroops("1.5000000")).toBe(15_000_000n);
  });

  it("handles large amounts", () => {
    // 1 billion USDC in stroops
    expect(toStroops("1000000000.0000000")).toBe(10_000_000_000_000_000n);
  });

  it("handles max fractional precision (7 digits)", () => {
    expect(toStroops("0.1234567")).toBe(1_234_567n);
    expect(toStroops("1.9999999")).toBe(19_999_999n);
  });

  // Error cases
  it("throws on more than 7 decimal places", () => {
    expect(() => toStroops("1.00000001")).toThrow(/7/);
    expect(() => toStroops("0.12345678")).toThrow();
  });

  it("throws on empty string", () => {
    expect(() => toStroops("")).toThrow();
  });

  it("throws on whitespace", () => {
    expect(() => toStroops(" 1.0")).toThrow();
    expect(() => toStroops("1.0 ")).toThrow();
  });

  it("throws on exponential notation", () => {
    expect(() => toStroops("1e7")).toThrow();
    expect(() => toStroops("1E7")).toThrow();
  });

  it("throws on multiple decimal points", () => {
    expect(() => toStroops("1.0.0")).toThrow();
  });

  it("throws on non-numeric input", () => {
    expect(() => toStroops("abc")).toThrow();
    expect(() => toStroops("1.abc")).toThrow();
    expect(() => toStroops("-1.0")).toThrow(); // negative sign in integer part
  });

  it("throws on non-string input", () => {
    // @ts-expect-error — intentionally wrong type for testing
    expect(() => toStroops(1.5)).toThrow(/string/);
    // @ts-expect-error
    expect(() => toStroops(null)).toThrow(/string/);
  });

  // Round-trip compatibility: toStroops and fromStroops are inverses
  it("round-trips with fromStroops", () => {
    const cases = ["0.0000000", "1.0000000", "25.5000000", "0.0000001", "1000000000.0000000"];
    for (const c of cases) {
      expect(fromStroops(toStroops(c))).toBe(c);
    }
  });
});

describe("fromStroops", () => {
  it("converts 10_000_000n to '1.0000000'", () => {
    expect(fromStroops(10_000_000n)).toBe("1.0000000");
  });

  it("converts 1n to '0.0000001'", () => {
    expect(fromStroops(1n)).toBe("0.0000001");
  });

  it("converts 0n to '0.0000000'", () => {
    expect(fromStroops(0n)).toBe("0.0000000");
  });

  it("converts a number as well as bigint", () => {
    expect(fromStroops(10_000_000)).toBe("1.0000000");
    expect(fromStroops(0)).toBe("0.0000000");
  });

  it("always produces exactly 7 decimal places", () => {
    const result = fromStroops(1_000_000n); // 0.1 USDC
    expect(result).toBe("0.1000000");
    const parts = result.split(".");
    expect(parts[1]).toHaveLength(7);
  });

  it("handles large amounts", () => {
    expect(fromStroops(10_000_000_000_000_000n)).toBe("1000000000.0000000");
  });

  it("throws on negative values", () => {
    expect(() => fromStroops(-1n)).toThrow();
    expect(() => fromStroops(-1)).toThrow();
  });
});
