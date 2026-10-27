/**
 * Shared types for the Antioch Escrow TypeScript client.
 */

/** Configuration required to instantiate an EscrowClient. */
export interface EscrowClientConfig {
  /** The on-chain contract ID (a Stellar account address, starts with C). */
  contractId: string;
  /** Soroban-compatible RPC URL, e.g. https://soroban-testnet.stellar.org */
  rpcUrl: string;
  /** Network passphrase, e.g. "Test SDF Network ; September 2015" */
  networkPassphrase: string;
  /** Optional source account for building fee-bump transactions. */
  sourceAccount?: string;
}

/** Arguments for building a `fund` transaction. */
export interface FundArgs {
  /** The donor's public key (G…). */
  donor: string;
  /** Amount in stroops (smallest token unit, i128 as bigint). */
  amount: bigint;
}

/** Arguments for building an `approve_milestone` transaction. */
export interface ApproveMilestoneArgs {
  /** The approver's public key (G…). */
  approver: string;
  /** Zero-based milestone index. */
  index: number;
}

/** Arguments for building a `release_milestone` transaction. */
export interface ReleaseMilestoneArgs {
  /** Zero-based milestone index. */
  index: number;
}

/** Arguments for building a `refund` transaction. */
export interface RefundArgs {
  /** The donor's public key (G…). */
  donor: string;
}

/** Result of any build* method — a base64-encoded unsigned transaction envelope XDR. */
export interface BuildResult {
  /** Base64-encoded XDR of an unsigned transaction envelope. Sign and submit with your wallet. */
  xdr: string;
}

/** The escrow lifecycle state as returned by `state()`. */
export type EscrowState = "Uninitialized" | "Active" | "Completed" | "Expired";

/** Data for a single milestone as returned by `readMilestone()`. */
export interface MilestoneData {
  /** Milestone amount in stroops. */
  amount: bigint;
  /** Whether this milestone has been released to the beneficiary. */
  released: boolean;
}

/** Snapshot of all escrow state from a single RPC simulation. */
export interface EscrowSnapshot {
  state: EscrowState;
  fundedTotal: bigint;
  releasedTotal: bigint;
  milestoneCount: number;
}
