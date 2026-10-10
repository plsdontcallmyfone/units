// Instructions the API already knows how to prepare (buys, cranks, season steps) come from the units
// API's prepare routes, decompiled back into instructions so the runtime can wrap and re-sign them
// under its own payer. The API never sees a key: it gets public addresses and returns unsigned bytes.
import { ComputeBudgetProgram, Connection, PublicKey, TransactionMessage, VersionedTransaction, type AddressLookupTableAccount, type TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { checkApiUrl } from './guard.ts';

export interface Router {
  /** Instructions of a prepare route (without compute budget instructions). */
  prepare(route: string, body: Record<string, unknown>): Promise<{ instructions: TransactionInstruction[]; tables: AddressLookupTableAccount[] }>;
  /** Token base units a buy of `lamportsIn` would return to `owner` now (simulated, nothing sent). */
  quoteBuy(owner: PublicKey, mint: PublicKey, lamportsIn: bigint): Promise<bigint>;
  /** Lamports a sell of `tokensIn` would return to `owner` now (simulated, nothing sent). */
  quoteSell(owner: PublicKey, mint: PublicKey, tokensIn: bigint): Promise<bigint>;
}

export class RouterError extends Error {}

/** Prepare routes the cranker role may call (all permissionless). */
export const CRANK_ROUTES = [
  'settle/prepare', 'seasons/open/prepare', 'seasons/finalize/prepare', 'prize/split/prepare', 'proposals/finalize/prepare', 'votes/close/prepare', 'agents/bonds/resolve/prepare',
  // R-2: the war steps (05 sections 7 to 9) and the economy's permissionless cranks.
  'war/siege/prepare', 'war/counter-strike/prepare', 'war/raze/prepare', 'book/crank/prepare', 'proposals/close/prepare', 'loyalty/reslot/prepare',
  // D-4 (17-randomness): revealing a loot roll is permissionless (the revealer pays the item's rent).
  'rolls/reveal/prepare',
] as const;

export class ApiRouter implements Router {
  readonly apiUrl: string;
  readonly conn: Connection;
  constructor(apiUrl: string, conn: Connection) { this.apiUrl = checkApiUrl(apiUrl).replace(/\/+$/, ''); this.conn = conn; }

  private async call(route: string, body: Record<string, unknown>): Promise<VersionedTransaction[]> {
    const res = await fetch(`${this.apiUrl}/v1/${route}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body), signal: AbortSignal.timeout(30_000) });
    const text = await res.text();
    if (!res.ok) throw new RouterError(`${route}: ${res.status} ${text.slice(0, 300)}`);
    const j = JSON.parse(text) as { transactions?: { transaction: string }[] };
    return (j.transactions ?? []).map((t) => VersionedTransaction.deserialize(Buffer.from(t.transaction, 'base64')));
  }

  async prepare(route: string, body: Record<string, unknown>) {
    const txs = await this.call(route, body);
    if (txs.length !== 1) throw new RouterError(`${route}: expected one transaction, got ${txs.length}`);
    const msg = txs[0]!.message;
    const tables: AddressLookupTableAccount[] = [];
    for (const l of msg.addressTableLookups) {
      const t = (await this.conn.getAddressLookupTable(l.accountKey, { commitment: 'confirmed' })).value;
      if (!t) throw new RouterError(`lookup table ${l.accountKey.toBase58()} not found`);
      tables.push(t);
    }
    const ixs = TransactionMessage.decompile(msg, { addressLookupTableAccounts: tables }).instructions;
    return { instructions: ixs.filter((i) => !i.programId.equals(ComputeBudgetProgram.programId)), tables };
  }

  async quoteBuy(owner: PublicKey, mint: PublicKey, lamportsIn: bigint): Promise<bigint> {
    const [tx] = await this.call('buy/prepare', { owner: owner.toBase58(), mint: mint.toBase58(), amount: lamportsIn.toString(), minOut: '1' });
    if (!tx) throw new RouterError('buy/prepare returned nothing');
    const holding = hookwars.holdingAddr(mint, owner);
    const before = await this.conn.getAccountInfo(holding, 'confirmed');
    const pre = before ? (hookwars.holdingCodec.decode(before.data).amount as bigint) : 0n;
    const sim = await this.conn.simulateTransaction(tx, { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed', accounts: { encoding: 'base64', addresses: [holding.toBase58()] } });
    if (sim.value.err) throw new RouterError(`quote simulation failed: ${JSON.stringify(sim.value.err)}`);
    const acct = sim.value.accounts?.[0];
    if (!acct) throw new RouterError('quote simulation returned no holding');
    const post = hookwars.holdingCodec.decode(Buffer.from(acct.data[0] ?? '', 'base64')).amount as bigint;
    return post - pre;
  }

  async quoteSell(owner: PublicKey, mint: PublicKey, tokensIn: bigint): Promise<bigint> {
    const [tx] = await this.call('sell/prepare', { owner: owner.toBase58(), mint: mint.toBase58(), amount: tokensIn.toString(), minOut: '0' });
    if (!tx) throw new RouterError('sell/prepare returned nothing');
    const pre = BigInt(await this.conn.getBalance(owner, 'confirmed'));
    const sim = await this.conn.simulateTransaction(tx, { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed', accounts: { encoding: 'base64', addresses: [owner.toBase58()] } });
    if (sim.value.err) throw new RouterError(`quote simulation failed: ${JSON.stringify(sim.value.err)}`);
    const acct = sim.value.accounts?.[0];
    if (!acct) throw new RouterError('quote simulation returned no account');
    return BigInt(acct.lamports) - pre;
  }
}
