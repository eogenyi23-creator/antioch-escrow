/**
 * @antioch/escrow-client
 *
 * TypeScript client for the Antioch Escrow Soroban contract.
 * Builds unsigned transaction XDR and reads state. Never holds keys.
 */

export { EscrowClient, TESTNET_PASSPHRASE, MAINNET_PASSPHRASE } from "./client";
export { toStroops, fromStroops, DECIMALS } from "./amounts";
export { decodeError, decodeErrorFromResult } from "./errors";
export type {
  EscrowClientConfig,
  FundArgs,
  ApproveMilestoneArgs,
  ReleaseMilestoneArgs,
  RefundArgs,
  BuildResult,
  EscrowState,
  MilestoneData,
  EscrowSnapshot,
} from "./types";
export type { ErrorCode, DecodedError } from "./errors";
