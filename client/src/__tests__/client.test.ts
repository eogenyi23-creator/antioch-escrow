/**
 * Tests for EscrowClient.build* methods.
 *
 * These tests verify that each build method produces a valid, non-empty
 * base64-encoded transaction envelope XDR. They do NOT hit an RPC endpoint
 * (no network required) — the build methods construct the transaction locally.
 *
 * We decode each XDR with stellar-sdk to assert:
 * - The transaction is well-formed (no throw on decode).
 * - The single operation is an invokeHostFunction.
 * - The encoded contract function name is correct.
 * - The argument count is correct.
 */

import { Networks, Transaction } from "@stellar/stellar-sdk";
import { EscrowClient, TESTNET_PASSPHRASE } from "../client";

// ---------------------------------------------------------------------------
// Test fixtures — generated with Keypair.random(); no real funds involved.
// ---------------------------------------------------------------------------

/** A valid Strkey C contract address (32-byte hash of 0xAB). */
const CONTRACT_ID = "CCV2XK5LVOV2XK5LVOV2XK5LVOV2XK5LVOV2XK5LVOV2XK5LVOV2XMCW";

/** A valid Strkey G address for use as donor/approver in tests. */
const VALID_DONOR =
  "GCDNBJRFMWOWG6Q5MMYYD2GM3Y77JQGQR45PRG23EL2KGAGH6JUJ7IQZ";

/** A second valid Strkey G address for use as approver in tests. */
const VALID_APPROVER =
  "GDHFHOOZTEBDV5KQHMKXKIX3KZHCWEFT7KUCQRRFWCDYR5UNSOZMMWCF";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create a client pointed at the test contract. */
function makeClient(): EscrowClient {
  return new EscrowClient({
    contractId: CONTRACT_ID,
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: TESTNET_PASSPHRASE,
  });
}

/**
 * Decode a base64 XDR envelope, assert it is an invokeHostFunction transaction,
 * and return the decoded invoke-contract details.
 *
 * stellar-sdk decodes the operation to a plain JS object:
 *   { type: 'invokeHostFunction', func: { type: 'invokeHostFunction', invokeContract: { functionName, contractAddress, args } }, auth: [] }
 */
function decodeInvokeContract(xdrBase64: string): {
  functionName: string;
  argCount: number;
} {
  const tx = new Transaction(xdrBase64, Networks.TESTNET);
  expect(tx.operations).toHaveLength(1);

  const op = tx.operations[0];
  expect(op.type).toBe("invokeHostFunction");

  // The decoded operation func object — cast to access invokeContract.
  // stellar-sdk 17 decodes it as a plain object (not an XDR instance).
  const func = (op as { func: { invokeContract: { functionName: { toString(): string }; args: unknown[] } } }).func;
  const ic = func.invokeContract;

  return {
    functionName: ic.functionName.toString(),
    argCount: ic.args.length,
  };
}

// ---------------------------------------------------------------------------
// buildFund
// ---------------------------------------------------------------------------

describe("EscrowClient — buildFund", () => {
  it("returns an object with a non-empty xdr string", async () => {
    const { xdr } = await makeClient().buildFund({
      donor: VALID_DONOR,
      amount: 10_000_000n,
    });
    expect(typeof xdr).toBe("string");
    expect(xdr.length).toBeGreaterThan(50);
  });

  it("produces valid, decodable base64 XDR", async () => {
    const { xdr } = await makeClient().buildFund({
      donor: VALID_DONOR,
      amount: 10_000_000n,
    });
    expect(() => new Transaction(xdr, Networks.TESTNET)).not.toThrow();
  });

  it("encodes the 'fund' contract function name", async () => {
    const { xdr } = await makeClient().buildFund({
      donor: VALID_DONOR,
      amount: 10_000_000n,
    });
    const { functionName } = decodeInvokeContract(xdr);
    expect(functionName).toBe("fund");
  });

  it("encodes exactly 2 arguments (donor, amount)", async () => {
    const { xdr } = await makeClient().buildFund({
      donor: VALID_DONOR,
      amount: 250_000_000n,
    });
    const { argCount } = decodeInvokeContract(xdr);
    expect(argCount).toBe(2);
  });

  it("produces different XDR for different amounts", async () => {
    const client = makeClient();
    const { xdr: xdr1 } = await client.buildFund({ donor: VALID_DONOR, amount: 1n });
    const { xdr: xdr2 } = await client.buildFund({ donor: VALID_DONOR, amount: 2n });
    expect(xdr1).not.toBe(xdr2);
  });

  it("produces different XDR for different donors", async () => {
    const client = makeClient();
    const { xdr: xdr1 } = await client.buildFund({ donor: VALID_DONOR, amount: 1n });
    const { xdr: xdr2 } = await client.buildFund({ donor: VALID_APPROVER, amount: 1n });
    expect(xdr1).not.toBe(xdr2);
  });
});

// ---------------------------------------------------------------------------
// buildApproveMilestone
// ---------------------------------------------------------------------------

describe("EscrowClient — buildApproveMilestone", () => {
  it("returns an object with a non-empty xdr string", async () => {
    const { xdr } = await makeClient().buildApproveMilestone({
      approver: VALID_APPROVER,
      index: 0,
    });
    expect(typeof xdr).toBe("string");
    expect(xdr.length).toBeGreaterThan(50);
  });

  it("produces valid, decodable base64 XDR", async () => {
    const { xdr } = await makeClient().buildApproveMilestone({
      approver: VALID_APPROVER,
      index: 0,
    });
    expect(() => new Transaction(xdr, Networks.TESTNET)).not.toThrow();
  });

  it("encodes 'approve_milestone' as the contract function name", async () => {
    const { xdr } = await makeClient().buildApproveMilestone({
      approver: VALID_APPROVER,
      index: 0,
    });
    const { functionName } = decodeInvokeContract(xdr);
    expect(functionName).toBe("approve_milestone");
  });

  it("encodes exactly 2 arguments (approver, index)", async () => {
    const { xdr } = await makeClient().buildApproveMilestone({
      approver: VALID_APPROVER,
      index: 1,
    });
    const { argCount } = decodeInvokeContract(xdr);
    expect(argCount).toBe(2);
  });

  it("produces different XDR for different milestone indices", async () => {
    const client = makeClient();
    const { xdr: xdr0 } = await client.buildApproveMilestone({
      approver: VALID_APPROVER,
      index: 0,
    });
    const { xdr: xdr1 } = await client.buildApproveMilestone({
      approver: VALID_APPROVER,
      index: 1,
    });
    expect(xdr0).not.toBe(xdr1);
  });
});

// ---------------------------------------------------------------------------
// buildReleaseMilestone
// ---------------------------------------------------------------------------

describe("EscrowClient — buildReleaseMilestone", () => {
  it("returns an object with a non-empty xdr string", async () => {
    const { xdr } = await makeClient().buildReleaseMilestone({ index: 0 });
    expect(typeof xdr).toBe("string");
    expect(xdr.length).toBeGreaterThan(50);
  });

  it("produces valid, decodable base64 XDR", async () => {
    const { xdr } = await makeClient().buildReleaseMilestone({ index: 0 });
    expect(() => new Transaction(xdr, Networks.TESTNET)).not.toThrow();
  });

  it("encodes 'release_milestone' as the contract function name", async () => {
    const { xdr } = await makeClient().buildReleaseMilestone({ index: 0 });
    const { functionName } = decodeInvokeContract(xdr);
    expect(functionName).toBe("release_milestone");
  });

  it("encodes exactly 1 argument (index)", async () => {
    const { xdr } = await makeClient().buildReleaseMilestone({ index: 2 });
    const { argCount } = decodeInvokeContract(xdr);
    expect(argCount).toBe(1);
  });

  it("produces different XDR for different indices", async () => {
    const client = makeClient();
    const { xdr: xdr0 } = await client.buildReleaseMilestone({ index: 0 });
    const { xdr: xdr3 } = await client.buildReleaseMilestone({ index: 3 });
    expect(xdr0).not.toBe(xdr3);
  });
});

// ---------------------------------------------------------------------------
// buildRefund
// ---------------------------------------------------------------------------

describe("EscrowClient — buildRefund", () => {
  it("returns an object with a non-empty xdr string", async () => {
    const { xdr } = await makeClient().buildRefund({ donor: VALID_DONOR });
    expect(typeof xdr).toBe("string");
    expect(xdr.length).toBeGreaterThan(50);
  });

  it("produces valid, decodable base64 XDR", async () => {
    const { xdr } = await makeClient().buildRefund({ donor: VALID_DONOR });
    expect(() => new Transaction(xdr, Networks.TESTNET)).not.toThrow();
  });

  it("encodes 'refund' as the contract function name", async () => {
    const { xdr } = await makeClient().buildRefund({ donor: VALID_DONOR });
    const { functionName } = decodeInvokeContract(xdr);
    expect(functionName).toBe("refund");
  });

  it("encodes exactly 1 argument (donor)", async () => {
    const { xdr } = await makeClient().buildRefund({ donor: VALID_DONOR });
    const { argCount } = decodeInvokeContract(xdr);
    expect(argCount).toBe(1);
  });

  it("produces different XDR for different donors", async () => {
    const client = makeClient();
    const { xdr: xdr1 } = await client.buildRefund({ donor: VALID_DONOR });
    const { xdr: xdr2 } = await client.buildRefund({ donor: VALID_APPROVER });
    expect(xdr1).not.toBe(xdr2);
  });
});

// ---------------------------------------------------------------------------
// EscrowClient config
// ---------------------------------------------------------------------------

describe("EscrowClient — config", () => {
  it("accepts a custom networkPassphrase and builds without error", async () => {
    const client = new EscrowClient({
      contractId: CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
    });
    const { xdr } = await client.buildFund({
      donor: VALID_DONOR,
      amount: 10_000_000n,
    });
    expect(xdr).toBeTruthy();
  });

  it("uses TESTNET_PASSPHRASE constant correctly", () => {
    expect(TESTNET_PASSPHRASE).toBe("Test SDF Network ; September 2015");
  });
});
