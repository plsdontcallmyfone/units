// Validate, then build. Each proposed action is checked by deterministic code against the agent
// config, the latest directive, the on-chain policy and (for trades) the policy engine; only then
// are its instructions built. Vault actions are wrapped in `hookwars_agents::spend`, so the vault,
// not the agent key, is the signer and the chain's own limits apply on top of these checks.
import { createHash } from 'node:crypto';
import { PublicKey, type AddressLookupTableAccount, type TransactionInstruction } from '@solana/web3.js';
import { hookwars, token } from '@hookwars/sdk';
import { roleOf, type Action } from './actions.ts';
import type { AgentsConfigView, ChainReader, PassportView, PolicyView } from './chain.ts';
import type { AgentConfig } from './config.ts';
import { commitInstruction, postInstruction, type Effective, type MemoParams } from './directive.ts';
import { get, memoInstruction, messageBytes, messageRef, obj, type Message, type Obj, type Value } from './memo.ts';
import { BANNED_WORDS, checkTrade, observe, type Book, type Identity, type TradeRequest } from './policy.ts';
import { provenanceValue, type Provenance } from './provenance.ts';
import type { Router } from './router.ts';
import { within, type AgentState } from './state.ts';

export interface TickContext {
  now: number;
  agentKey: PublicKey;
  passport: PassportView;
  vault: PublicKey;
  policy: PolicyView;
  agentsConfig: AgentsConfigView;
  memoParams: MemoParams;
  eff: Effective;
  cfg: AgentConfig;
  state: AgentState;
  identity: Identity;
  vaultLamports: bigint;
}

export interface Plan {
  action: Action;
  label: string;
  instructions: TransactionInstruction[];
  tables: AddressLookupTableAccount[];
  memo: Message;
  /** For trades: what the book needs after the fill. */
  trade?: { req: TradeRequest; holding: PublicKey };
}

export type Outcome = { ok: true; plan: Plan } | { ok: false; refusals: string[] };

const MESSAGE_ID = /^$|^[1-9A-HJ-NP-Za-km-z]{64,88}:\d{1,3}$/;
const DAY = 86_400;

/** Every string inside a memo value. */
function strings(v: Value, out: string[] = []): string[] {
  if (typeof v === 'string') out.push(v);
  else if (Array.isArray(v)) v.forEach((x) => strings(x, out));
  else if (v !== null && typeof v === 'object') v.obj.forEach(([k, x]) => { out.push(k); strings(x, out); });
  return out;
}

/** Naming rules and safety for anything an agent publishes. */
export function textProblems(v: Value): string[] {
  const out: string[] = [];
  for (const s of strings(v)) {
    if (s.includes('\u2014')) out.push('an em dash');
    const words = s.toLowerCase().split(/[^a-z]+/);
    const bad = BANNED_WORDS.find((w) => words.includes(w));
    if (bad) out.push(`the word "${bad}"`);
    // A 64-byte secret in base58 is 87 or 88 characters; a message id is a signature of that length followed by `:index`.
    if (/(?<![1-9A-HJ-NP-Za-km-z])[1-9A-HJ-NP-Za-km-z]{87,88}(?![1-9A-HJ-NP-Za-km-z:])/.test(s)) out.push('something that looks like a secret key');
  }
  return [...new Set(out)];
}

/** The spend wrapper: the vault signs inside the program; at the top level its metas are not signers. */
export function wrapSpend(ix: TransactionInstruction, agentKey: PublicKey, passport: PublicKey, vault: PublicKey): TransactionInstruction {
  return hookwars.idlIx('agents', 'spend', {
    agentKey, config: hookwars.agentsConfigAddress(), passport, policy: hookwars.policyAddress(passport), vault, targetProgram: ix.programId,
  }, { data: ix.data }, ix.keys.map((k) => (k.pubkey.equals(vault) ? { ...k, isSigner: false } : k)));
}

const signsAs = (ix: TransactionInstruction, key: PublicKey) => ix.keys.some((k) => k.isSigner && k.pubkey.equals(key));

/** Rebuilds a prepared vault-owner transaction for the agent: holdings paid by the agent key, the rest through `spend`. */
export function vaultInstructions(ixs: TransactionInstruction[], agentKey: PublicKey, passport: PublicKey, vault: PublicKey): { instructions: TransactionInstruction[]; foreignSigner: boolean } {
  const out: TransactionInstruction[] = [];
  let foreignSigner = false;
  for (const ix of ixs) {
    const mint = ix.keys[1]?.pubkey;
    if (mint && ix.programId.equals(hookwars.TOKEN_ID) && signsAs(ix, vault)) {
      const h = token.createHolding(vault, mint, vault);
      if (h.data.equals(ix.data) && h.keys.length === ix.keys.length && h.keys.every((k, i) => k.pubkey.equals(ix.keys[i]!.pubkey))) { out.push(token.createHolding(agentKey, mint, vault)); continue; }
    }
    if (signsAs(ix, vault)) { out.push(wrapSpend(ix, agentKey, passport, vault)); continue; }
    if (ix.keys.some((k) => k.isSigner && !k.pubkey.equals(agentKey))) foreignSigner = true;
    out.push(ix);
  }
  return { instructions: out, foreignSigner };
}

function actionMemo(ctx: TickContext, act: string, text: string, pv: Provenance, extra: [string, Value][] = []): Message {
  return { kind: 'status', from: ctx.passport.address.toBase58(), to: '*', thread: '', re: '', body: obj([['act', act], ['text', text], ...extra, ['pv', provenanceValue(pv)]]), expiresAt: 0n };
}

function withPv(body: Obj, pv: Provenance): Obj { return obj([...body.obj.filter(([k]) => k !== 'pv'), ['pv', provenanceValue(pv)]]); }

/** Remaining spend under the on-chain policy today (the chain enforces it exactly; this avoids sure failures). */
function spendRoom(p: PolicyView, now: number, lamports: bigint): string | null {
  if (p.frozen) return 'the policy is frozen';
  if (lamports > p.perActionLamports) return `${lamports} lamports is over the policy per-action limit ${p.perActionLamports}`;
  const spent = BigInt(now) >= p.dayStart + BigInt(DAY) ? 0n : p.spentToday;
  if (spent + lamports > p.perDayLamports) return `${lamports} lamports would pass the policy per-day limit ${p.perDayLamports} (spent ${spent})`;
  return null;
}

function targetAllowed(ctx: TickContext, program: PublicKey): string | null {
  if (!ctx.policy.targets.some((t) => t.equals(program))) return `program ${program.toBase58()} is not a target in the agent's policy`;
  if (!ctx.agentsConfig.targets.some((t) => t.equals(program))) return `program ${program.toBase58()} is not a target the agents config allows`;
  return null;
}

export interface Deps { chain: ChainReader; router: Router }

export async function planAction(action: Action, ctx: TickContext, deps: Deps, pv: Provenance): Promise<Outcome> {
  const refuse = (...r: string[]): Outcome => ({ ok: false, refusals: r });
  if (ctx.eff.halted) return refuse(ctx.eff.halted);
  const role = roleOf(action);
  if (!ctx.eff.roles.includes(role)) return refuse(`the ${role} role is not allowed (config and directive ${ctx.eff.seq})`);
  const agent = ctx.agentKey;
  const passport = ctx.passport.address;
  const vault = ctx.vault;
  const memoLimit = () => {
    const hour = within(ctx.state.memoTimes, ctx.now, 3_600).length;
    const day = within(ctx.state.memoTimes, ctx.now, DAY).length;
    if (hour >= ctx.cfg.memo.maxPerHour) return `memo rate limit: ${hour} in the last hour`;
    if (day >= ctx.cfg.memo.maxPerDay) return `memo rate limit: ${day} in the last day`;
    return null;
  };
  const done = (label: string, instructions: TransactionInstruction[], memo: Message, tables: AddressLookupTableAccount[] = [], trade?: Plan['trade']): Outcome => {
    const bytes = messageBytes(memo);
    if (bytes.length > ctx.memoParams.memoMaxBytes) return refuse(`the memo is ${bytes.length} bytes, over memo_max_bytes ${ctx.memoParams.memoMaxBytes}`);
    const problems = textProblems(memo.body);
    if (problems.length) return refuse(`the memo contains ${problems.join(', ')}`);
    const ixs = [...instructions, memoInstruction(memo, [agent])];
    if (ctx.cfg.memo.postage) ixs.push(postInstruction(agent, passport, ctx.agentsConfig.feeCollector, createHash('sha256').update(bytes).digest()));
    return { ok: true, plan: { action, label, instructions: ixs, tables, memo, ...(trade ? { trade } : {}) } };
  };

  switch (action.type) {
    case 'create_item': {
      const a = ctx.cfg.author!;
      const allowed = ctx.eff.templates ?? a.templates;
      const r: string[] = [];
      if (!allowed.includes(action.templateId) || !a.templates.includes(action.templateId)) r.push(`template ${action.templateId} is not allowed`);
      if (action.royaltyBps > a.maxRoyaltyBps) r.push(`royalty ${action.royaltyBps} bps is over the configured ${a.maxRoyaltyBps}`);
      if (within(ctx.state.itemTimes, ctx.now, DAY).length >= a.maxItemsPerDay) r.push('item limit for the day reached');
      if (action.params.length > hookwars.PARAM_FIELDS) r.push(`a template takes at most ${hookwars.PARAM_FIELDS} params`);
      if (r.length) return refuse(...r);
      const minted = await deps.chain.itemsMinted();
      // Unused parameter fields are zero.
      const params = [...action.params, ...Array<number>(hookwars.PARAM_FIELDS - action.params.length).fill(0)];
      const ix = hookwars.createItem(agent, action.templateId, params, action.royaltyBps, minted, vault);
      const itemMint = hookwars.itemMintAddress(minted);
      return done('create item', [ix], actionMemo(ctx, 'create_item', `created item ${itemMint.toBase58()} from template ${action.templateId}, royalty ${action.royaltyBps} bps, held by the agent vault`, pv));
    }
    case 'list_item': {
      const m = ctx.cfg.market!;
      const itemMint = new PublicKey(action.itemMint);
      const r: string[] = [];
      if (action.priceLamports <= 0n) r.push('price must be above zero');
      if (action.priceLamports > m.maxListPriceLamports) r.push(`price is over the configured ${m.maxListPriceLamports}`);
      if ((await deps.chain.tokenBalance(itemMint, vault)) < 1n) r.push('the agent vault does not hold this item');
      const t = targetAllowed(ctx, hookwars.MARKET_ID); if (t) r.push(t);
      if (ctx.policy.frozen) r.push('the policy is frozen');
      if (r.length) return refuse(...r);
      const inner = hookwars.marketList(vault, hookwars.itemAddress(itemMint), itemMint, action.priceLamports, action.expiresAt);
      return done('list item', [wrapSpend(inner, agent, passport, vault)], actionMemo(ctx, 'list_item', `listed item ${itemMint.toBase58()} for ${action.priceLamports} lamports`, pv));
    }
    case 'offer_lease': {
      const m = ctx.cfg.market!;
      const itemMint = new PublicKey(action.itemMint);
      const r: string[] = [];
      if (action.feeLamports > m.maxLeaseFeeLamports) r.push(`lease fee is over the configured ${m.maxLeaseFeeLamports}`);
      if ((await deps.chain.tokenBalance(itemMint, vault)) < 1n) r.push('the agent vault does not hold this item');
      const t = targetAllowed(ctx, hookwars.MARKET_ID); if (t) r.push(t);
      if (ctx.policy.frozen) r.push('the policy is frozen');
      if (r.length) return refuse(...r);
      const inner = hookwars.marketOfferLease(vault, hookwars.itemAddress(itemMint), itemMint, new PublicKey(action.tokenMint), action.slot, action.rentBps, action.feeLamports, action.termSecs);
      return done('offer lease', [wrapSpend(inner, agent, passport, vault)], actionMemo(ctx, 'offer_lease', `offered item ${itemMint.toBase58()} for lease on slot ${action.slot}, fee ${action.feeLamports} lamports, rent ${action.rentBps} bps, term ${action.termSecs} s`, pv));
    }
    case 'message': {
      const lim = memoLimit(); if (lim) return refuse(lim);
      if (!MESSAGE_ID.test(action.thread) || !MESSAGE_ID.test(action.re)) return refuse('thread and re must be empty or a message id (signature:index)');
      if (action.kind === 'accept' || action.kind === 'ack') {
        const ref = get(action.body, 'ref');
        if (typeof ref !== 'string' || !ref || !MESSAGE_ID.test(ref)) return refuse(`${action.kind} needs a body.ref message id`);
      }
      const memo: Message = { kind: action.kind, from: passport.toBase58(), to: action.to, thread: action.thread, re: action.re, body: withPv(action.body, pv), expiresAt: action.expiresAt };
      return done(`message ${action.kind}`, [], memo);
    }
    case 'commit': {
      const reference = messageRef(action.messageId);
      return done('commit', [commitInstruction(agent, passport, reference, Buffer.from(action.hash, 'hex'))], actionMemo(ctx, 'commit', `committed to message ${action.messageId}`, pv));
    }
    case 'post_bond': {
      const ix = hookwars.agentsPostBond(agent, passport, new PublicKey(action.proposalA), new PublicKey(action.proposalB), new PublicKey(action.treatyItem));
      return done('post bond', [ix], actionMemo(ctx, 'post_bond', `posted a treaty bond on proposals ${action.proposalA} and ${action.proposalB}`, pv));
    }
    case 'crank': {
      if (!ctx.cfg.cranks!.routes.includes(action.route)) return refuse(`route ${action.route} is not in cranks.routes`);
      let prepared;
      try { prepared = await deps.router.prepare(action.route, action.body); } catch (e) { return refuse(`prepare failed: ${(e as Error).message}`); }
      const foreign = prepared.instructions.flatMap((ix) => ix.keys.filter((k) => k.isSigner && !k.pubkey.equals(agent)));
      if (foreign.length) return refuse('the prepared crank needs a signer other than the agent key');
      return done(`crank ${action.route}`, prepared.instructions, actionMemo(ctx, 'crank', `cranked ${action.route}`, pv), prepared.tables);
    }
    case 'trade': {
      const caps = ctx.eff.caps;
      if (!caps) return refuse('no trade caps configured');
      if (ctx.eff.universe && !ctx.eff.universe.includes(action.mint)) return refuse('the token is outside the trading universe');
      const mint = new PublicKey(action.mint);
      const facts = await deps.chain.mintFacts(mint);
      if (!facts) return refuse('no such token');
      const sell = action.side === 'sell';
      let quotedOut: bigint;
      try { quotedOut = sell ? await deps.router.quoteSell(vault, mint, action.amountIn) : await deps.router.quoteBuy(vault, mint, action.amountIn); } catch (e) { return refuse(`quote failed: ${(e as Error).message}`); }
      const req: TradeRequest = { side: action.side, mint: action.mint, amountIn: action.amountIn, quotedOut, minOut: action.minOut, reason: action.reason };
      const book: Book = observe(ctx.state.book!, { vaultLamports: ctx.vaultLamports, values: {} }, caps, ctx.now);
      const r = checkTrade(book, caps, req, facts, ctx.identity, ctx.now).map((x) => `${x.code}: ${x.detail}`);
      // A buy spends lamports from the vault; a sell spends tokens (the chain's tracked-mint limits apply).
      if (!sell) { const room = spendRoom(ctx.policy, ctx.now, action.amountIn); if (room) r.push(room); } else if (ctx.policy.frozen) r.push('the policy is frozen');
      if (r.length) return refuse(...r);
      const route = sell ? 'sell/prepare' : 'buy/prepare';
      let prepared;
      try { prepared = await deps.router.prepare(route, { owner: vault.toBase58(), mint: action.mint, amount: action.amountIn.toString(), minOut: action.minOut.toString() }); } catch (e) { return refuse(`prepare failed: ${(e as Error).message}`); }
      const v = vaultInstructions(prepared.instructions, agent, passport, vault);
      if (v.foreignSigner) return refuse(`the prepared ${action.side} needs a signer other than the agent and its vault`);
      for (const ix of v.instructions) if (ix.programId.equals(hookwars.AGENTS_ID)) { const t = targetAllowed(ctx, new PublicKey(ix.keys[5]!.pubkey)); if (t) return refuse(t); }
      const text = sell
        ? `sold ${action.amountIn} base units of ${mint.toBase58()}, floor ${action.minOut} lamports, quote ${quotedOut}`
        : `bought ${mint.toBase58()} with ${action.amountIn} lamports, floor ${action.minOut}, quote ${quotedOut}`;
      const memo = actionMemo(ctx, 'trade', text, pv, [['reason', action.reason.trim()]]);
      return done(sell ? 'sell' : 'buy', v.instructions, memo, prepared.tables, { req, holding: hookwars.holdingAddr(mint, vault) });
    }
    case 'set_access': {
      // R-4: `set_access` (11 section 1.1, spec 13 E-1) is deferred in the armory. The action is
      // checked against the directive now and is built once the armory IDL carries the instruction.
      const k = ctx.eff.constraints;
      const r: string[] = [];
      if ((k.allowedAccessModes & (1 << action.mode)) === 0) r.push(`access mode ${action.mode} is not allowed by the directive`);
      if (action.licencePrice > k.maxLicencePrice) r.push(`licence price ${action.licencePrice} is over the directive's ${k.maxLicencePrice}`);
      const itemMint = new PublicKey(action.itemMint);
      if ((await deps.chain.tokenBalance(itemMint, vault)) < 1n) r.push('the agent vault does not hold this item');
      if (ctx.policy.frozen) r.push('the policy is frozen');
      if (r.length) return refuse(...r);
      if (!hookwars.coderOf('armory').idl.instructions.some((i) => i.name === 'set_access')) return refuse('set_access is not in the armory program yet (spec 13 E-1 is deferred); the action activates when the regenerated IDL carries it');
      let inner: TransactionInstruction;
      try {
        inner = hookwars.idlIx('armory', 'set_access', { holder: vault, owner: vault, item: hookwars.itemAddress(itemMint), itemMint, holding: hookwars.holdingAddr(itemMint, vault) }, { mode: action.mode, licencePrice: action.licencePrice, price: action.licencePrice });
      } catch (e) { return refuse(`set_access could not be built from the IDL: ${(e as Error).message}`); }
      return done('set access', [wrapSpend(inner, agent, passport, vault)], actionMemo(ctx, 'set_access', `set access mode ${action.mode} on item ${itemMint.toBase58()}, licence ${action.licencePrice} lamports`, pv));
    }
    case 'status': {
      const lim = memoLimit(); if (lim) return refuse(lim);
      if (!ctx.state.pendingFacts.length) return refuse('nothing happened since the last status: status memos report real events only');
      const voice = action.voice.trim();
      if (/\d/.test(voice)) return refuse('the voice line may not contain digits: numbers come only from chain facts');
      const facts: string[] = [];
      let len = 0;
      for (const f of ctx.state.pendingFacts) { if (len + f.text.length > 240) break; facts.push(f.text); len += f.text.length + 2; }
      const memo: Message = { kind: 'status', from: passport.toBase58(), to: '*', thread: '', re: '', body: obj([['text', facts.join('; ')], ['voice', voice], ['pv', provenanceValue(pv)]]), expiresAt: 0n };
      return done('status', [], memo);
    }
  }
}
