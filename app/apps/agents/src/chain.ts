// What the runtime reads from the chain, behind one interface so the loop runs the same against a
// cluster (`RpcChain`) and against a mocked chain in tests (`MockChain` in mock-chain.ts).
import { Connection, PublicKey } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { decodeDirective, decodeMemoConfig, directiveAddress, memoConfigAddress, type DirectiveAccount, type MemoParams } from './directive.ts';
import { MEMO_PROGRAM_ID } from './memo.ts';
import type { MintFacts } from './policy.ts';
import { checkRulesUri } from './guard.ts';

export const PASSPORT_ACTIVE = 0;

export interface PassportView { address: PublicKey; operator: PublicKey; index: number; agentKey: PublicKey; name: string; status: number; proof: number }
export interface PolicyView { frozen: boolean; perActionLamports: bigint; perDayLamports: bigint; dayStart: bigint; spentToday: bigint; targets: PublicKey[] }
export interface AgentsConfigView { feeCollector: PublicKey; targets: PublicKey[] }

/** A real event the agent took part in, as plain facts (no model involved). */
export interface Fact { signature: string; name: string; text: string; slot: number }

export interface ChainReader {
  /** Cluster time (unix seconds). */
  now(): Promise<number>;
  passport(address: PublicKey): Promise<PassportView | null>;
  policy(passport: PublicKey): Promise<PolicyView | null>;
  agentsConfig(): Promise<AgentsConfigView | null>;
  memoParams(): Promise<MemoParams | null>;
  directive(passport: PublicKey, seq: number): Promise<DirectiveAccount | null>;
  /** The bytes of the directive memo in the transaction that created the `Directive` account. */
  directiveMemo(passport: PublicKey, seq: number): Promise<Buffer | null>;
  /** The rules document (size-capped, https from a public address only). */
  fetchRules(uri: string): Promise<Buffer>;
  itemsMinted(): Promise<bigint>;
  /** The creator of a token and the authors of the items equipped on it. */
  mintFacts(mint: PublicKey): Promise<MintFacts | null>;
  /** Every key of the operator: its passports, their agent keys and vaults. */
  operatorKeys(operator: PublicKey): Promise<string[]>;
  lamports(address: PublicKey): Promise<bigint>;
  tokenBalance(mint: PublicKey, owner: PublicKey): Promise<bigint>;
  /** Facts newer than `cursor` (a signature) from transactions touching `addresses`. */
  recentFacts(addresses: PublicKey[], cursor: string | null): Promise<{ facts: Fact[]; cursor: string | null }>;
}

const RULES_MAX_BYTES = 64 * 1024;

export async function fetchCapped(uri: string, maxBytes = RULES_MAX_BYTES, timeoutMs = 10_000): Promise<Buffer> {
  // Review 3 L-9: https, public addresses only (a shared runtime must not reach its own network).
  await checkRulesUri(uri);
  const res = await fetch(uri, { signal: AbortSignal.timeout(timeoutMs), redirect: 'error' });
  if (!res.ok) throw new Error(`rules_uri answered ${res.status}`);
  const len = Number(res.headers.get('content-length') ?? '0');
  if (len > maxBytes) throw new Error('the rules document is too large');
  const b = Buffer.from(await res.arrayBuffer());
  if (b.length > maxBytes) throw new Error('the rules document is too large');
  return b;
}

const passportCodec = hookwars.idlAccountCodec<{ operator: PublicKey; index: number; agentKey: PublicKey; name: string; status: number; proof: number }>('agents', 'Passport');
const policyCodec = hookwars.idlAccountCodec<{ frozen: boolean; perActionLamports: bigint; perDayLamports: bigint; dayStart: bigint; spentToday: bigint; targets: PublicKey[] }>('agents', 'Policy');
const configCodec = hookwars.idlAccountCodec<{ feeCollector: PublicKey; targets: PublicKey[] }>('agents', 'AgentsConfig');
const itemCodec = hookwars.idlAccountCodec<{ author: PublicKey }>('armory', 'Item');

/** Offset of `Passport.operator`: discriminator, version, bump. */
const PASSPORT_OPERATOR_OFFSET = 10;

export class RpcChain implements ChainReader {
  readonly conn: Connection;
  constructor(conn: Connection) { this.conn = conn; }

  async now(): Promise<number> {
    const slot = await this.conn.getSlot('confirmed');
    return (await this.conn.getBlockTime(slot)) ?? Math.floor(Date.now() / 1000);
  }
  async passport(address: PublicKey): Promise<PassportView | null> {
    const info = await this.conn.getAccountInfo(address, 'confirmed');
    if (!info) return null;
    const p = passportCodec.decode(info.data);
    return { address, operator: p.operator, index: p.index, agentKey: p.agentKey, name: p.name, status: p.status, proof: p.proof };
  }
  async policy(passport: PublicKey): Promise<PolicyView | null> {
    const info = await this.conn.getAccountInfo(hookwars.policyAddress(passport), 'confirmed');
    if (!info) return null;
    const p = policyCodec.decode(info.data);
    return { frozen: p.frozen, perActionLamports: p.perActionLamports, perDayLamports: p.perDayLamports, dayStart: p.dayStart, spentToday: p.spentToday, targets: p.targets };
  }
  async agentsConfig(): Promise<AgentsConfigView | null> {
    const info = await this.conn.getAccountInfo(hookwars.agentsConfigAddress(), 'confirmed');
    if (!info) return null;
    const c = configCodec.decode(info.data);
    return { feeCollector: c.feeCollector, targets: c.targets };
  }
  async memoParams(): Promise<MemoParams | null> {
    const info = await this.conn.getAccountInfo(memoConfigAddress(), 'confirmed');
    return info ? decodeMemoConfig(info.data) : null;
  }
  async directive(passport: PublicKey, seq: number): Promise<DirectiveAccount | null> {
    const info = await this.conn.getAccountInfo(directiveAddress(passport, seq), 'confirmed');
    return info ? decodeDirective(info.data) : null;
  }
  async directiveMemo(passport: PublicKey, seq: number): Promise<Buffer | null> {
    // The creating transaction is the oldest one touching the account.
    const sigs = await this.conn.getSignaturesForAddress(directiveAddress(passport, seq), { limit: 1000 }, 'confirmed');
    const oldest = sigs.filter((s) => !s.err).at(-1);
    if (!oldest) return null;
    const tx = await this.conn.getTransaction(oldest.signature, { maxSupportedTransactionVersion: 0, commitment: 'confirmed' });
    if (!tx) return null;
    const keys = tx.transaction.message.getAccountKeys({ accountKeysFromLookups: tx.meta?.loadedAddresses ?? null });
    for (const ix of tx.transaction.message.compiledInstructions) {
      if (keys.get(ix.programIdIndex)?.equals(MEMO_PROGRAM_ID)) {
        const data = Buffer.from(ix.data);
        if (data.includes(Buffer.from('"k":"directive"'))) return data;
      }
    }
    return null;
  }
  fetchRules(uri: string): Promise<Buffer> { return fetchCapped(uri); }
  async itemsMinted(): Promise<bigint> {
    const info = await this.conn.getAccountInfo(hookwars.armoryConfigAddress(), 'confirmed');
    if (!info) throw new Error('no armory config on this cluster');
    return hookwars.armoryConfigCodec.decode(info.data).itemsMinted;
  }
  async mintFacts(mint: PublicKey): Promise<MintFacts | null> {
    const info = await this.conn.getAccountInfo(mint, 'confirmed');
    if (!info) return null;
    const m = hookwars.decodeSlotMint(info.data);
    const items = hookwars.activeSlots(m).map((s) => s.item).filter((k) => !k.equals(PublicKey.default));
    const infos = items.length ? await this.conn.getMultipleAccountsInfo(items, 'confirmed') : [];
    const authors: string[] = [];
    for (const i of infos) { if (!i) continue; try { authors.push(itemCodec.decode(i.data).author.toBase58()); } catch { /* not an armory item */ } }
    return { creator: m.creator.toBase58(), itemAuthors: authors };
  }
  async operatorKeys(operator: PublicKey): Promise<string[]> {
    const accts = await this.conn.getProgramAccounts(hookwars.AGENTS_ID, {
      commitment: 'confirmed',
      filters: [{ memcmp: { offset: 0, bytes: passportCodec.disc.toString('base64'), encoding: 'base64' } }, { memcmp: { offset: PASSPORT_OPERATOR_OFFSET, bytes: operator.toBase58() } }],
    });
    const out = new Set([operator.toBase58()]);
    for (const a of accts) {
      try {
        const p = passportCodec.decode(a.account.data);
        out.add(a.pubkey.toBase58()); out.add(p.agentKey.toBase58()); out.add(hookwars.agentVaultAddress(a.pubkey).toBase58());
      } catch { /* skip */ }
    }
    return [...out];
  }
  async lamports(address: PublicKey): Promise<bigint> { return BigInt(await this.conn.getBalance(address, 'confirmed')); }
  async tokenBalance(mint: PublicKey, owner: PublicKey): Promise<bigint> {
    const info = await this.conn.getAccountInfo(hookwars.holdingAddr(mint, owner), 'confirmed');
    if (!info) return 0n;
    return hookwars.holdingCodec.decode(info.data).amount as bigint;
  }
  async recentFacts(addresses: PublicKey[], cursor: string | null): Promise<{ facts: Fact[]; cursor: string | null }> {
    const seen = new Map<string, number>();
    for (const a of addresses) {
      const sigs = await this.conn.getSignaturesForAddress(a, { limit: 25, ...(cursor ? { until: cursor } : {}) }, 'confirmed');
      for (const s of sigs) if (!s.err) seen.set(s.signature, s.slot);
    }
    const ordered = [...seen.entries()].sort((x, y) => x[1] - y[1]);
    const facts: Fact[] = [];
    for (const [sig, slot] of ordered) {
      const tx = await this.conn.getTransaction(sig, { maxSupportedTransactionVersion: 0, commitment: 'confirmed' });
      if (!tx?.meta) continue;
      const keys = tx.transaction.message.getAccountKeys({ accountKeysFromLookups: tx.meta.loadedAddresses ?? null });
      const accountKeys = [...keys.staticAccountKeys, ...(keys.accountKeysFromLookups?.writable ?? []), ...(keys.accountKeysFromLookups?.readonly ?? [])].map((k) => k.toBase58());
      const inner = (tx.meta.innerInstructions ?? []).flatMap((x) => x.instructions.map((i) => ({ programIdIndex: i.programIdIndex, accounts: i.accounts, data: i.data })));
      const { events } = hookwars.decodeTransactionEvents({ accountKeys, inner, logs: tx.meta.logMessages ?? [] });
      for (const e of events) facts.push({ signature: sig, slot, name: `${e.program}.${e.name}`, text: factText(e.name, e.data) });
    }
    return { facts, cursor: ordered.at(-1)?.[0] ?? cursor };
  }
}

/** A short, deterministic line for an event: its name and up to four plain fields. */
export function factText(name: string, data: Record<string, unknown>): string {
  const parts: string[] = [];
  for (const [k, v] of Object.entries(data)) {
    if (parts.length >= 4) break;
    if (k === 'ts' || k === 'bump') continue;
    if (typeof v === 'bigint' || typeof v === 'number') parts.push(`${k} ${v}`);
    else if (typeof v === 'string' && v.length <= 44) parts.push(`${k} ${v.length > 12 ? v.slice(0, 4) + '..' + v.slice(-4) : v}`);
    else if (v instanceof PublicKey) { const s = v.toBase58(); parts.push(`${k} ${s.slice(0, 4)}..${s.slice(-4)}`); }
  }
  return parts.length ? `${name}: ${parts.join(', ')}` : name;
}

