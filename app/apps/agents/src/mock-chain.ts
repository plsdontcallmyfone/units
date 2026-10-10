// An in-memory chain and router for tests and `--mock` dry runs: the same interfaces the loop uses
// against a cluster, with every account set by hand.
import { createHash } from 'node:crypto';
import { PublicKey, type AddressLookupTableAccount, type TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import type { AgentsConfigView, ChainReader, Fact, PassportView, PolicyView } from './chain.ts';
import { messageBytes, directiveMessage } from './memo.ts';
import type { DirectiveAccount, DirectiveConstraints, MemoParams } from './directive.ts';
import type { MintFacts } from './policy.ts';
import type { Router } from './router.ts';

export class MockChain implements ChainReader {
  time = 1_800_000_000;
  passports = new Map<string, PassportView>();
  policies = new Map<string, PolicyView>();
  config: AgentsConfigView | null = null;
  memo: MemoParams | null = null;
  directives = new Map<string, DirectiveAccount>();
  directiveMemos = new Map<string, Buffer>();
  rules = new Map<string, Buffer>();
  minted = 0n;
  mints = new Map<string, MintFacts>();
  operators = new Map<string, string[]>();
  balances = new Map<string, bigint>();
  tokens = new Map<string, bigint>();
  facts: Fact[] = [];

  async now() { return this.time; }
  async passport(a: PublicKey) { return this.passports.get(a.toBase58()) ?? null; }
  async policy(p: PublicKey) { return this.policies.get(p.toBase58()) ?? null; }
  async agentsConfig() { return this.config; }
  async memoParams() { return this.memo; }
  async directive(p: PublicKey, seq: number) { return this.directives.get(`${p.toBase58()}:${seq}`) ?? null; }
  async directiveMemo(p: PublicKey, seq: number) { return this.directiveMemos.get(`${p.toBase58()}:${seq}`) ?? null; }
  async fetchRules(uri: string) { const r = this.rules.get(uri); if (!r) throw new Error('rules_uri answered 404'); return r; }
  async itemsMinted() { return this.minted; }
  async mintFacts(m: PublicKey) { return this.mints.get(m.toBase58()) ?? null; }
  async operatorKeys(op: PublicKey) { return this.operators.get(op.toBase58()) ?? [op.toBase58()]; }
  async lamports(a: PublicKey) { return this.balances.get(a.toBase58()) ?? 0n; }
  async tokenBalance(mint: PublicKey, owner: PublicKey) { return this.tokens.get(`${mint.toBase58()}:${owner.toBase58()}`) ?? 0n; }
  async recentFacts(_a: PublicKey[], cursor: string | null) {
    const i = cursor ? this.facts.findIndex((f) => f.signature === cursor) + 1 : 0;
    const facts = this.facts.slice(i);
    return { facts, cursor: facts.at(-1)?.signature ?? cursor };
  }

  /** Posts a directive the way the operator's transaction would: memo bytes, their hash, the account. */
  postDirective(passport: PublicKey, seq: number, constraints: DirectiveConstraints, rulesUri: string, rulesDoc: string): { memo: Buffer; h: string } {
    const doc = Buffer.from(rulesDoc, 'utf8');
    const h = createHash('sha256').update(doc).digest('hex');
    const memo = messageBytes(directiveMessage(passport.toBase58(), BigInt(seq), rulesUri, h));
    const prev = this.directives.get(`${passport.toBase58()}:${seq - 1}`);
    if (prev) prev.supersededBy = seq;
    this.directives.set(`${passport.toBase58()}:${seq}`, { passport, seq, memoHash: createHash('sha256').update(memo).digest(), constraints, postedAt: BigInt(this.time), supersededBy: null, bump: 255 });
    this.directiveMemos.set(`${passport.toBase58()}:${seq}`, memo);
    this.rules.set(rulesUri, doc);
    return { memo, h };
  }
}

export class MockRouter implements Router {
  quotes = new Map<string, bigint>();
  prepared: { route: string; body: Record<string, unknown> }[] = [];
  builders = new Map<string, (body: Record<string, unknown>) => TransactionInstruction[]>();
  async prepare(route: string, body: Record<string, unknown>): Promise<{ instructions: TransactionInstruction[]; tables: AddressLookupTableAccount[] }> {
    this.prepared.push({ route, body });
    const b = this.builders.get(route);
    if (!b) throw new Error(`${route}: 404 No such prepare route.`);
    return { instructions: b(body), tables: [] };
  }
  async quoteBuy(_owner: PublicKey, mint: PublicKey, _lamportsIn: bigint): Promise<bigint> {
    const q = this.quotes.get(mint.toBase58());
    if (q === undefined) throw new Error('no quote');
    return q;
  }
  sellQuotes = new Map<string, bigint>();
  async quoteSell(_owner: PublicKey, mint: PublicKey, _tokensIn: bigint): Promise<bigint> {
    const q = this.sellQuotes.get(mint.toBase58());
    if (q === undefined) throw new Error('no quote');
    return q;
  }
}

/** Addresses of one agent, as the loop derives them. */
export function agentAddresses(operator: PublicKey, index: number) {
  const passport = hookwars.passportAddress(operator, index);
  return { passport, vault: hookwars.agentVaultAddress(passport), policy: hookwars.policyAddress(passport) };
}
