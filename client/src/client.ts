/**
 * EscrowClient — builds unsigned Soroban transaction XDR and reads state
 * from the Antioch Escrow contract.
 *
 * This client NEVER holds private keys. Every `build*` method returns a
 * base64-encoded unsigned transaction envelope. Your wallet (browser, hardware,
 * or CLI) signs and submits it.
 *
 * Read helpers call `simulateTransaction` on the configured RPC endpoint and
 * decode the return value without submitting anything on-chain.
 *
 * @example
 * ```ts
 * import { EscrowClient, toStroops } from "@antioch/escrow-client";
 *
 * const escrow = new EscrowClient({
 *   contractId: "CABC...",
 *   rpcUrl: "https://soroban-testnet.stellar.org",
 *   networkPassphrase: "Test SDF Network ; September 2015",
 * });
 *
 * // Build an unsigned fund transaction — sign it with your wallet.
 * const { xdr } = await escrow.buildFund({
 *   donor: donorPublicKey,
 *   amount: toStroops("25.0000000"),
 * });
 * ```
 */

import {
  Account,
  Contract,
  nativeToScVal,
  Networks,
  rpc as StellarRpc,
  scValToNative,
  TransactionBuilder,
  xdr,
} from "@stellar/stellar-sdk";

import type {
  ApproveMilestoneArgs,
  BuildResult,
  EscrowClientConfig,
  EscrowSnapshot,
  EscrowState,
  FundArgs,
  MilestoneData,
  RefundArgs,
  ReleaseMilestoneArgs,
} from "./types";

/** Default transaction timeout in seconds. */
const DEFAULT_TIMEOUT_SECONDS = 30;

/** Default base fee in stroops (100 stroops = 0.00001 XLM). */
const DEFAULT_BASE_FEE = "100";

/**
 * A null source account used when building transactions without a known
 * source. The fee and sequence number are irrelevant for an unsigned XDR that
 * the wallet will re-sequence before signing.
 *
 * Using the zero-address ensures we never hit the Horizon validation that
 * checks whether the source account exists, since we are not submitting.
 */
const NULL_ACCOUNT_ID =
  "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

/**
 * EscrowClient provides:
 * - `build*` methods that return unsigned XDR for each state-changing function.
 * - `read*` methods that simulate read-only view functions and return decoded values.
 */
export class EscrowClient {
  private readonly contractId: string;
  private readonly networkPassphrase: string;
  private readonly rpcUrl: string;

  constructor(config: EscrowClientConfig) {
    this.contractId = config.contractId;
    this.networkPassphrase = config.networkPassphrase;
    this.rpcUrl = config.rpcUrl;
  }

  // ---------------------------------------------------------------------------
  // Build helpers — construct an unsigned transaction for each entry point.
  // ---------------------------------------------------------------------------

  /**
   * Build an unsigned `fund(donor, amount)` transaction.
   *
   * The donor must sign and submit this transaction. The contract will call
   * `donor.require_auth()` before transferring tokens.
   */
  async buildFund(args: FundArgs): Promise<BuildResult> {
    const op = this.contract().call(
      "fund",
      nativeToScVal(args.donor, { type: "address" }),
      nativeToScVal(args.amount, { type: "i128" })
    );
    return this.buildXdr(op);
  }

  /**
   * Build an unsigned `approve_milestone(approver, index)` transaction.
   *
   * The approver must sign and submit this transaction.
   */
  async buildApproveMilestone(args: ApproveMilestoneArgs): Promise<BuildResult> {
    const op = this.contract().call(
      "approve_milestone",
      nativeToScVal(args.approver, { type: "address" }),
      nativeToScVal(args.index, { type: "u32" })
    );
    return this.buildXdr(op);
  }

  /**
   * Build an unsigned `release_milestone(index)` transaction.
   *
   * This function requires no authentication — anyone can call it.
   * The resulting XDR can be submitted by any account.
   */
  async buildReleaseMilestone(args: ReleaseMilestoneArgs): Promise<BuildResult> {
    const op = this.contract().call(
      "release_milestone",
      nativeToScVal(args.index, { type: "u32" })
    );
    return this.buildXdr(op);
  }

  /**
   * Build an unsigned `refund(donor)` transaction.
   *
   * The donor must sign and submit this transaction.
   */
  async buildRefund(args: RefundArgs): Promise<BuildResult> {
    const op = this.contract().call(
      "refund",
      nativeToScVal(args.donor, { type: "address" })
    );
    return this.buildXdr(op);
  }

  // ---------------------------------------------------------------------------
  // Read helpers — simulate view functions via RPC and decode results.
  // ---------------------------------------------------------------------------

  /**
   * Read the current escrow state.
   *
   * Simulates the `state()` view function. Returns one of:
   * "Uninitialized" | "Active" | "Completed" | "Expired".
   */
  async readState(): Promise<EscrowState> {
    const result = await this.simulate("state", []);
    const native = scValToNative(result);
    return this.decodeState(native);
  }

  /**
   * Read the total tokens funded so far (in stroops).
   */
  async readFundedTotal(): Promise<bigint> {
    const result = await this.simulate("funded_total", []);
    return BigInt(scValToNative(result) as string | number | bigint);
  }

  /**
   * Read the total tokens released to the beneficiary (in stroops).
   */
  async readReleasedTotal(): Promise<bigint> {
    const result = await this.simulate("released_total", []);
    return BigInt(scValToNative(result) as string | number | bigint);
  }

  /**
   * Read a single milestone's amount (in stroops) and released flag.
   */
  async readMilestone(index: number): Promise<MilestoneData> {
    const result = await this.simulate("milestone", [
      nativeToScVal(index, { type: "u32" }),
    ]);
    const [amount, released] = scValToNative(result) as [
      bigint | string | number,
      boolean,
    ];
    return {
      amount: BigInt(amount),
      released: Boolean(released),
    };
  }

  /**
   * Read the total number of milestones.
   */
  async readMilestoneCount(): Promise<number> {
    const result = await this.simulate("milestone_count", []);
    return Number(scValToNative(result));
  }

  /**
   * Read the number of approvals recorded for a milestone.
   */
  async readApprovals(index: number): Promise<number> {
    const result = await this.simulate("approvals", [
      nativeToScVal(index, { type: "u32" }),
    ]);
    return Number(scValToNative(result));
  }

  /**
   * Read a donor's total contribution (in stroops).
   */
  async readDonorContribution(donor: string): Promise<bigint> {
    const result = await this.simulate("donor_contribution", [
      nativeToScVal(donor, { type: "address" }),
    ]);
    return BigInt(scValToNative(result) as string | number | bigint);
  }

  /**
   * Read a snapshot of the top-level escrow state in a single RPC call.
   * Useful for dashboards that need state + totals together.
   *
   * Note: This makes four separate simulate calls. For production use,
   * consider batching with a multi-call helper.
   */
  async readSnapshot(): Promise<EscrowSnapshot> {
    const [state, fundedTotal, releasedTotal, milestoneCount] =
      await Promise.all([
        this.readState(),
        this.readFundedTotal(),
        this.readReleasedTotal(),
        this.readMilestoneCount(),
      ]);
    return { state, fundedTotal, releasedTotal, milestoneCount };
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private contract(): Contract {
    return new Contract(this.contractId);
  }

  /** Build an unsigned transaction envelope XDR from a Soroban operation. */
  private buildXdr(op: xdr.Operation): BuildResult {
    // Use a null-sequence account so wallets can re-sequence before signing.
    const account = new Account(NULL_ACCOUNT_ID, "0");
    const tx = new TransactionBuilder(account, {
      fee: DEFAULT_BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(op)
      .setTimeout(DEFAULT_TIMEOUT_SECONDS)
      .build();

    return { xdr: tx.toEnvelope().toXDR("base64") };
  }

  /** Simulate a view function and return the raw ScVal result. */
  private async simulate(
    method: string,
    args: xdr.ScVal[]
  ): Promise<xdr.ScVal> {
    const server = new StellarRpc.Server(this.rpcUrl, { allowHttp: true });

    // Use the source account from config if provided, otherwise fall back to null account.
    const account = new Account(NULL_ACCOUNT_ID, "0");
    const op = this.contract().call(method, ...args);
    const tx = new TransactionBuilder(account, {
      fee: DEFAULT_BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(op)
      .setTimeout(DEFAULT_TIMEOUT_SECONDS)
      .build();

    const simResult = await server.simulateTransaction(tx);

    if (!StellarRpc.Api.isSimulationSuccess(simResult)) {
      const errorMsg =
        "error" in simResult ? String(simResult.error) : "Unknown simulation error";
      throw new Error(`Contract simulation failed: ${errorMsg}`);
    }

    const result = simResult.result;
    if (!result) {
      throw new Error(`Simulation returned no result for ${method}()`);
    }

    return xdr.ScVal.fromXDR(result.retval.toXDR());
  }

  /** Decode a native JS value (from scValToNative on an enum ScVal) to EscrowState. */
  private decodeState(native: unknown): EscrowState {
    // Soroban enum types decode to their variant name as a string.
    if (typeof native === "string") {
      switch (native) {
        case "Uninitialized":
        case "Active":
        case "Completed":
        case "Expired":
          return native;
        default:
          throw new Error(`Unknown escrow state: "${native}"`);
      }
    }
    // Some SDK versions decode enum variants as objects like { Active: {} }.
    if (native !== null && typeof native === "object") {
      const key = Object.keys(native as object)[0] as EscrowState;
      return key;
    }
    throw new Error(`Cannot decode escrow state from: ${JSON.stringify(native)}`);
  }
}

/** Re-export the known Stellar testnet and mainnet passphrases for convenience. */
export const TESTNET_PASSPHRASE = Networks.TESTNET;
export const MAINNET_PASSPHRASE = Networks.PUBLIC;
