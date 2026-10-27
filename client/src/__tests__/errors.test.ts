import { decodeError, decodeErrorFromResult } from "../errors";
import type { DecodedError } from "../errors";

describe("decodeError", () => {
  it("decodes code 16 as FundingCapExceeded", () => {
    const result = decodeError(16);
    expect(result.code).toBe(16);
    expect(result.name).toBe("FundingCapExceeded");
    expect(result.message).toContain("exceed");
  });

  it("decodes code 17 as ThresholdNotMet", () => {
    const result = decodeError(17);
    expect(result.code).toBe(17);
    expect(result.name).toBe("ThresholdNotMet");
    expect(result.message).toContain("threshold");
  });

  it("decodes code 18 as InvalidExpiry", () => {
    const result = decodeError(18);
    expect(result.code).toBe(18);
    expect(result.name).toBe("InvalidExpiry");
    expect(result.message).toContain("expiry");
  });

  it("decodes all known error codes", () => {
    for (let code = 1; code <= 18; code++) {
      const result = decodeError(code);
      expect(result.code).toBe(code);
      expect(result.name).toBeTruthy();
      expect(result.message).toBeTruthy();
    }
  });

  it("decodes code 1 as NotInitialized", () => {
    const result = decodeError(1);
    expect(result.name).toBe("NotInitialized");
  });

  it("decodes code 15 as Overflow", () => {
    const result = decodeError(15);
    expect(result.name).toBe("Overflow");
  });

  it("throws on unknown error code", () => {
    expect(() => decodeError(99)).toThrow(/Unknown/);
    expect(() => decodeError(0)).toThrow(/Unknown/);
  });
});

describe("decodeErrorFromResult", () => {
  it("decodes a numeric code directly", () => {
    const result = decodeErrorFromResult(16);
    expect(result).not.toBeNull();
    expect(result!.name).toBe("FundingCapExceeded");
  });

  it("decodes an object with a code property", () => {
    const result = decodeErrorFromResult({ code: 17 });
    expect(result).not.toBeNull();
    expect(result!.name).toBe("ThresholdNotMet");
  });

  it("decodes a nested error object", () => {
    const result = decodeErrorFromResult({ error: { code: 18 } });
    expect(result).not.toBeNull();
    expect(result!.name).toBe("InvalidExpiry");
  });

  it("extracts a number from a string", () => {
    const result = decodeErrorFromResult("error #16)");
    expect(result).not.toBeNull();
    expect(result!.name).toBe("FundingCapExceeded");
  });

  it("returns null for an unknown numeric code", () => {
    expect(decodeErrorFromResult(99)).toBeNull();
  });

  it("returns null for non-error values", () => {
    expect(decodeErrorFromResult(null)).toBeNull();
    expect(decodeErrorFromResult(undefined)).toBeNull();
    expect(decodeErrorFromResult("no number here")).toBeNull();
    expect(decodeErrorFromResult({ foo: "bar" })).toBeNull();
  });

  it("returns a DecodedError with all fields", () => {
    const result = decodeErrorFromResult(9) as DecodedError;
    expect(result.code).toBe(9);
    expect(result.name).toBe("InsufficientFunds");
    expect(result.message).toContain("insufficient");
  });
});
