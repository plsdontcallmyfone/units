// One tick of an agent: read its passport, policy and latest directive; collect the facts of what
// happened; ask the model; check each proposal deterministically; build, simulate and (only with
// --send) sign. The model never holds a key and never sees one.
import { PublicKey } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { ACTION_GUIDE, parseReply, type Action } from './actions.ts';
import { PASSPORT_ACTIVE, type ChainReader } from './chain.ts';
import type { AgentConfig } from './config.ts';
import { effective, parseRules, verifyDirectiveMemo, type DirectiveAccount, type Effective, type Rules } from './directive.ts';
import type { Logger } from './log.ts';
import type { Model, ModelRequest } from './models/types.ts';
import { planAction, type TickContext } from './plan.ts';
import { applyFill, clearHalt, emptyBook, equityOf, observe } from './policy.ts';
import { provenanceOf, type Provenance } from './provenance.ts';
import type { Router } from './router.ts';
import type { Sender } from './sender.ts';
import { within, type AgentState, type StateStore } from './state.ts';

export interface Runtime {
  cfg: AgentConfig;
  agentKey: PublicKey;
  model: Model;
  chain: ChainReader;
  router: Router;
  sender: Sender;
  store: StateStore;
  log: Logger;
  /** Called with every prompt and output so the operator can publish them (provenance log). */
  recordPrompt?: (rec: { provenance: Provenance; request: ModelRequest; output: string }) => void;
}

export interface ActionResult { type: Action['type']; ok: boolean; refusals?: string[]; label?: string; sent?: boolean; signature?: string | null; error?: string | null }

export interface TickReport {
  now: number;
  passport: string;
  directiveSeq: number | null;
  halted: string | null;
  provenance: Provenance | null;
  dropped: { index: number; why: string }[];
  results: ActionResult[];
}

/** Finds the latest directive, starting from the last one followed. */
export async function latestDirective(chain: ChainReader, passport: PublicKey, from: number | null): Promise<DirectiveAccount | null> {
  let seq = from ?? 0;
  let d = await chain.directive(passport, seq);
  if (!d) {
    if (seq === 0) return null;
    // The remembered one vanished (a reset cluster): start again from 0.
    seq = 0; d = await chain.directive(passport, 0);
    if (!d) return null;
  }
  for (;;) {
    const next = await chain.directive(passport, seq + 1);
    if (!next) return d;
    d = next; seq += 1;
  }
}

export function buildPrompt(cfg: AgentConfig, eff: Effective, rules: Rules, snapshot: Record<string, unknown>): ModelRequest {
  const system = [
    `You are ${cfg.name}, an agent on units, a Solana economy of hook items. You act only through the JSON actions below; code checks every proposal against your operator's directive and limits, and refuses anything outside them.`,
    cfg.persona ? `Voice and interests: ${cfg.persona}` : '',
    `Your roles: ${eff.roles.join(', ') || 'none'}.`,
    eff.notes ? `Your operator's directive ${eff.seq} says:\n${eff.notes}` : `Your operator's directive ${eff.seq} adds no notes.`,
    'Rules: spot only. Never put keys, private terms or personal data in a message. Never use the words yield, APR, APY, reflections, tax, bet or odds. Status lines state only what happened; numbers come from chain facts, not from you. Every trade needs a short public reason.',
    ACTION_GUIDE,
  ].filter(Boolean).join('\n\n');
  const user = `State (JSON):\n${JSON.stringify(snapshot, (_k, v) => (typeof v === 'bigint' ? v.toString() : v))}`;
  void rules;
  return { system, messages: [{ role: 'user', content: user }], maxTokens: cfg.maxTokens, temperature: cfg.temperature };
}

export async function tick(rt: Runtime): Promise<TickReport> {
  const { cfg, chain, log } = rt;
  const state: AgentState = rt.store.load();
  const now = await chain.now();
  const passportAddr = hookwars.passportAddress(cfg.operator, cfg.passportIndex);
  const vault = hookwars.agentVaultAddress(passportAddr);
  const report: TickReport = { now, passport: passportAddr.toBase58(), directiveSeq: null, halted: null, provenance: null, dropped: [], results: [] };
  const halt = (why: string): TickReport => { report.halted = why; rt.store.save(state); log.log('warn', 'halted', { why }); return report; };

  const passport = await chain.passport(passportAddr);
  if (!passport) return halt('no passport at the configured operator and index');
  if (!passport.agentKey.equals(rt.agentKey)) return halt('the passport names another agent key');
  if (!passport.operator.equals(cfg.operator)) return halt('the passport names another operator');
  if (passport.status !== PASSPORT_ACTIVE) return halt(`the passport status is ${passport.status}, not active`);
  const [policy, agentsConfig, memoParams] = await Promise.all([chain.policy(passportAddr), chain.agentsConfig(), chain.memoParams()]);
  if (!policy) return halt('the agent has no policy');
  if (!agentsConfig || !memoParams) return halt('the agents config or memo config is missing on this cluster');

  // The directive: account, memo bound by hash, rules bound by hash. Anything off means no actions.
  const d = await latestDirective(chain, passportAddr, state.directiveSeq);
  if (!d) return halt('no directive yet: the operator has not programmed this agent');
  report.directiveSeq = d.seq;
  let rules: Rules;
  try {
    const memo = await chain.directiveMemo(passportAddr, d.seq);
    if (!memo) return halt(`directive ${d.seq}: its memo was not found`);
    const body = verifyDirectiveMemo(memo, d, memoParams.memoMaxBytes);
    rules = parseRules(await chain.fetchRules(body.rulesUri), body.h);
  } catch (e) { return halt(`directive ${d.seq}: ${(e as Error).message}`); }
  state.directiveSeq = d.seq;
  const eff = effective({ roles: cfg.roles, caps: cfg.trade?.caps ?? null, universe: cfg.trade?.universe ?? null, templates: cfg.author?.templates ?? null }, d, rules);

  const operatorKeys = new Set(await chain.operatorKeys(cfg.operator));
  const vaultLamports = await chain.lamports(vault);
  state.book ??= emptyBook(now, vaultLamports);
  if (eff.clearHalt && state.clearedHaltSeq !== d.seq) { state.book = clearHalt(state.book, equityOf(state.book, { vaultLamports, values: {} })); state.clearedHaltSeq = d.seq; }
  if (eff.caps) state.book = observe(state.book, { vaultLamports, values: {} }, eff.caps, now);

  const f = await chain.recentFacts([vault, rt.agentKey], state.factsCursor);
  state.factsCursor = f.cursor;
  for (const x of f.facts) if (!state.pendingFacts.some((p) => p.signature === x.signature && p.text === x.text)) state.pendingFacts.push({ signature: x.signature, text: x.text });
  state.pendingFacts = state.pendingFacts.slice(-50);

  if (eff.halted) return halt(eff.halted);
  if (cfg.maxActionsPerTick === 0) { rt.store.save(state); return report; }

  const snapshot = {
    now, passport: passportAddr.toBase58(), vault: vault.toBase58(), directive: d.seq, roles: eff.roles,
    policy: { frozen: policy.frozen, perActionLamports: policy.perActionLamports, perDayLamports: policy.perDayLamports, spentToday: policy.spentToday },
    vaultLamports, positions: state.book.positions, tradeCaps: eff.caps, universe: eff.universe, templates: eff.templates,
    memoBudget: { lastHour: within(state.memoTimes, now, 3_600).length, maxPerHour: cfg.memo.maxPerHour, lastDay: within(state.memoTimes, now, 86_400).length, maxPerDay: cfg.memo.maxPerDay },
    crankRoutes: cfg.cranks?.routes ?? [], newFacts: state.pendingFacts.map((p) => p.text), halt: state.book.halt,
  };
  const req = buildPrompt(cfg, eff, rules, snapshot);
  let reply;
  try { reply = await rt.model.complete(req); } catch (e) { log.log('error', 'model-failed', { provider: rt.model.provider, model: rt.model.model, error: (e as Error).message }); rt.store.save(state); return report; }
  const pv = provenanceOf(req, reply);
  report.provenance = pv;
  rt.recordPrompt?.({ provenance: pv, request: req, output: reply.text });
  const parsed = parseReply(reply.text, cfg.maxActionsPerTick);
  report.dropped = parsed.dropped;
  for (const dr of parsed.dropped) log.log('warn', 'action-dropped', dr);

  for (const action of parsed.actions) {
    const ctx: TickContext = { now, agentKey: rt.agentKey, passport, vault, policy, agentsConfig, memoParams, eff, cfg, state, identity: { agentKey: rt.agentKey.toBase58(), vault: vault.toBase58(), operatorKeys }, vaultLamports };
    const out = await planAction(action, ctx, { chain, router: rt.router }, pv);
    if (!out.ok) { report.results.push({ type: action.type, ok: false, refusals: out.refusals }); log.log('info', 'action-refused', { type: action.type, refusals: out.refusals }); continue; }
    const p = out.plan;
    // A buy is measured in tokens received, a sell in lamports received by the vault.
    const measure = async (): Promise<bigint> => (!p.trade ? 0n : p.trade.req.side === 'sell' ? chain.lamports(vault) : chain.tokenBalance(new PublicKey(p.trade.req.mint), vault));
    const before = await measure();
    const res = await rt.sender.submit(p.instructions, p.tables);
    report.results.push({ type: action.type, ok: res.error === null, label: p.label, sent: res.sent, signature: res.signature, error: res.error });
    log.log(res.error ? 'warn' : 'info', 'action', { type: action.type, label: p.label, sent: res.sent, signature: res.signature, error: res.error, units: res.unitsConsumed });
    if (!res.sent || res.error) continue;
    if (action.type === 'message' || action.type === 'status') state.memoTimes.push(now);
    if (action.type === 'status') state.pendingFacts = [];
    if (action.type === 'create_item') state.itemTimes.push(now);
    if (p.trade) {
      const after = await measure();
      state.book = applyFill(state.book, p.trade.req, after - before, now);
    }
  }
  state.memoTimes = within(state.memoTimes, now, 86_400);
  state.itemTimes = within(state.itemTimes, now, 86_400);
  rt.store.save(state);
  return report;
}
