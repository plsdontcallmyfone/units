// Compiles a plan into one v0 transaction paid and signed by the agent key, simulates it, and only
// sends when the runtime was started with --send. Dry run is the default.
import { ComputeBudgetProgram, Connection, Keypair, TransactionMessage, VersionedTransaction, type AddressLookupTableAccount, type TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';

export const PACKET_BYTES = 1_232;
export const MAX_CU = 1_400_000;

export interface SendResult { sent: boolean; signature: string | null; unitsConsumed: number | null; logs: string[]; error: string | null; bytes: number; depthLimited?: boolean }

/** The deepest `Program <id> invoke [n]` the logs show (1 = top level; Solana allows 5). */
export function invokeDepth(logs: string[]): number {
  let max = 0;
  for (const l of logs) { const m = / invoke \[(\d+)\]$/.exec(l); if (m) max = Math.max(max, Number(m[1])); }
  return max;
}

/** Runtime gap R-5: whether a failure is the call depth limit (`spend` adds one level to a route). */
export function isDepthLimit(err: unknown, logs: string[]): boolean {
  const e = JSON.stringify(err ?? '');
  return /CallDepth/.test(e) || logs.some((l) => /call depth|CallDepth|invocation depth/i.test(l));
}

function simError(err: unknown, logs: string[]): { error: string; depthLimited: boolean } {
  if (isDepthLimit(err, logs)) return { error: `call depth limit: the route reached ${invokeDepth(logs)} program levels through spend, the most Solana runs (R-5); it cannot run from the vault`, depthLimited: true };
  return { error: `simulation failed: ${JSON.stringify(err)}`, depthLimited: false };
}

export interface Sender { submit(ixs: TransactionInstruction[], tables: AddressLookupTableAccount[]): Promise<SendResult> }

export function compile(payer: Keypair, ixs: TransactionInstruction[], tables: AddressLookupTableAccount[], blockhash: string, units: number, extra: Keypair[] = []): VersionedTransaction {
  const heap = ixs.some((i) => i.programId.equals(hookwars.SWAP_ID)) ? [ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 })] : [];
  const msg = new TransactionMessage({ payerKey: payer.publicKey, recentBlockhash: blockhash, instructions: [ComputeBudgetProgram.setComputeUnitLimit({ units }), ...heap, ...ixs] }).compileToV0Message(tables);
  const tx = new VersionedTransaction(msg);
  tx.sign([payer, ...extra]);
  return tx;
}

export class RpcSender implements Sender {
  readonly conn: Connection;
  readonly payer: Keypair;
  readonly send: boolean;
  readonly extraTables: AddressLookupTableAccount[];
  readonly extraSigners: Keypair[];
  constructor(conn: Connection, payer: Keypair, send: boolean, extraTables: AddressLookupTableAccount[] = [], extraSigners: Keypair[] = []) { this.conn = conn; this.payer = payer; this.send = send; this.extraTables = extraTables; this.extraSigners = extraSigners; }

  async submit(ixs: TransactionInstruction[], tables: AddressLookupTableAccount[]): Promise<SendResult> {
    const all = [...tables, ...this.extraTables];
    const { blockhash, lastValidBlockHeight } = await this.conn.getLatestBlockhash('confirmed');
    let tx = compile(this.payer, ixs, all, blockhash, MAX_CU, this.extraSigners);
    let bytes: number;
    try { bytes = tx.serialize().length; } catch { return { sent: false, signature: null, unitsConsumed: null, logs: [], error: 'the transaction does not fit in one packet', bytes: 0 }; }
    if (bytes > PACKET_BYTES) return { sent: false, signature: null, unitsConsumed: null, logs: [], error: `the transaction is ${bytes} bytes, over ${PACKET_BYTES}`, bytes };
    const sim = await this.conn.simulateTransaction(tx, { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed' });
    const logs = sim.value.logs ?? [];
    if (sim.value.err) return { sent: false, signature: null, unitsConsumed: sim.value.unitsConsumed ?? null, logs, bytes, ...simError(sim.value.err, logs) };
    const units = Math.min(MAX_CU, Math.ceil((sim.value.unitsConsumed ?? 200_000) * 1.15));
    if (!this.send) return { sent: false, signature: null, unitsConsumed: sim.value.unitsConsumed ?? null, logs, error: null, bytes };
    tx = compile(this.payer, ixs, all, blockhash, units, this.extraSigners);
    const signature = await this.conn.sendTransaction(tx, { skipPreflight: true, maxRetries: 3 });
    const conf = await this.conn.confirmTransaction({ signature, blockhash, lastValidBlockHeight }, 'confirmed');
    return { sent: true, signature, unitsConsumed: sim.value.unitsConsumed ?? null, logs, error: conf.value.err ? `failed on chain: ${JSON.stringify(conf.value.err)}` : null, bytes };
  }
}

/** Records what would be sent (tests and `--mock` dry runs). */
export class RecordingSender implements Sender {
  readonly submitted: { instructions: TransactionInstruction[]; tables: AddressLookupTableAccount[] }[] = [];
  async submit(ixs: TransactionInstruction[], tables: AddressLookupTableAccount[]): Promise<SendResult> {
    this.submitted.push({ instructions: ixs, tables });
    return { sent: false, signature: null, unitsConsumed: null, logs: [], error: null, bytes: 0 };
  }
}
