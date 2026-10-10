// Per-agent config file (JSON). It holds no secret: the agent key is a path to a keypair file read at
// start, and model keys come from the environment. Every limit is the operator's to set; the runtime
// ships no defaults for caps, rates or prices.
import { readFileSync } from 'node:fs';
import { dirname, isAbsolute, resolve } from 'node:path';
import { Keypair, PublicKey } from '@solana/web3.js';
import { ROLES, type Role } from './directive.ts';
import type { ModelSpec } from './models/registry.ts';
import type { TradeCaps } from './policy.ts';
import { CRANK_ROUTES } from './router.ts';

export interface AgentConfig {
  name: string;
  operator: PublicKey;
  passportIndex: number;
  agentKeypairPath: string;
  roles: Role[];
  model: ModelSpec;
  /** A few lines on the agent's voice and interests, given to the model (public in effect: it shapes public memos). */
  persona: string;
  tickSecs: number;
  maxActionsPerTick: number;
  maxTokens: number;
  temperature: number;
  memo: { maxPerHour: number; maxPerDay: number; postage: boolean };
  author: { templates: number[]; maxRoyaltyBps: number; maxItemsPerDay: number } | null;
  market: { maxListPriceLamports: bigint; maxLeaseFeeLamports: bigint } | null;
  trade: { caps: TradeCaps; universe: string[] | null } | null;
  cranks: { routes: string[] } | null;
  stateDir: string;
}

export class ConfigError extends Error {}

const need = (c: boolean, why: string): void => { if (!c) throw new ConfigError(why); };
const int = (x: unknown, what: string, min = 0, max = Number.MAX_SAFE_INTEGER): number => { need(typeof x === 'number' && Number.isSafeInteger(x) && x >= min && x <= max, `${what}: an integer from ${min} to ${max}`); return x as number; };
const big = (x: unknown, what: string): bigint => { need(typeof x === 'string' && /^\d+$/.test(x), `${what}: a decimal string of lamports`); return BigInt(x as string); };
const obj = (x: unknown, what: string): Record<string, unknown> => { need(!!x && typeof x === 'object' && !Array.isArray(x), `${what}: an object`); return x as Record<string, unknown>; };

export function parseConfig(j: unknown, baseDir = '.'): AgentConfig {
  const c = obj(j, 'config');
  need(typeof c.name === 'string' && /^[a-z0-9-]{1,32}$/.test(c.name as string), 'name: 1 to 32 lowercase letters, digits or dashes');
  let operator: PublicKey;
  try { operator = new PublicKey(c.operator as string); } catch { throw new ConfigError('operator: a base58 address'); }
  need(typeof c.agentKeypairPath === 'string', 'agentKeypairPath: the path of the agent keypair file');
  need(Array.isArray(c.roles) && (c.roles as unknown[]).every((r) => (ROLES as readonly unknown[]).includes(r)), `roles: a list of ${ROLES.join(', ')}`);
  const roles = [...new Set(c.roles as Role[])];
  const m = obj(c.model, 'model');
  need(typeof m.provider === 'string', 'model.provider: a provider id');
  const memo = obj(c.memo, 'memo');
  const has = (r: Role) => roles.includes(r);
  const rel = (p: string) => (isAbsolute(p) ? p : resolve(baseDir, p));
  const cfg: AgentConfig = {
    name: c.name as string,
    operator,
    passportIndex: int(c.passportIndex, 'passportIndex', 0, 0xffff_ffff),
    agentKeypairPath: rel(c.agentKeypairPath as string),
    roles,
    model: { provider: m.provider as string, ...(typeof m.model === 'string' ? { model: m.model } : {}) },
    persona: typeof c.persona === 'string' ? (c.persona as string).slice(0, 2_000) : '',
    tickSecs: int(c.tickSecs, 'tickSecs', 1),
    maxActionsPerTick: int(c.maxActionsPerTick, 'maxActionsPerTick', 0, 16),
    maxTokens: int(c.maxTokens, 'maxTokens', 1, 32_000),
    temperature: typeof c.temperature === 'number' && c.temperature >= 0 && c.temperature <= 2 ? c.temperature : 0,
    memo: { maxPerHour: int(memo.maxPerHour, 'memo.maxPerHour'), maxPerDay: int(memo.maxPerDay, 'memo.maxPerDay'), postage: memo.postage === true },
    author: null, market: null, trade: null, cranks: null,
    stateDir: rel(typeof c.stateDir === 'string' ? c.stateDir : `state/${c.name}`),
  };
  if (has('author')) {
    const a = obj(c.author, 'author (required for the author role)');
    need(Array.isArray(a.templates), 'author.templates: a list of template ids');
    cfg.author = { templates: (a.templates as unknown[]).map((t, i) => int(t, `author.templates[${i}]`, 0, 0xffff)), maxRoyaltyBps: int(a.maxRoyaltyBps, 'author.maxRoyaltyBps', 0, 10_000), maxItemsPerDay: int(a.maxItemsPerDay, 'author.maxItemsPerDay') };
  }
  if (has('market')) {
    const a = obj(c.market, 'market (required for the market role)');
    cfg.market = { maxListPriceLamports: big(a.maxListPriceLamports, 'market.maxListPriceLamports'), maxLeaseFeeLamports: big(a.maxLeaseFeeLamports, 'market.maxLeaseFeeLamports') };
  }
  if (has('trader')) {
    const t = obj(c.trade, 'trade (required for the trader role)');
    const k = obj(t.caps, 'trade.caps');
    cfg.trade = {
      caps: {
        maxPositionLamports: big(k.maxPositionLamports, 'trade.caps.maxPositionLamports'),
        maxTradeLamports: big(k.maxTradeLamports, 'trade.caps.maxTradeLamports'),
        dailyLossHaltLamports: big(k.dailyLossHaltLamports, 'trade.caps.dailyLossHaltLamports'),
        drawdownHaltBps: int(k.drawdownHaltBps, 'trade.caps.drawdownHaltBps', 0, 10_000),
        maxSlippageBps: int(k.maxSlippageBps, 'trade.caps.maxSlippageBps', 0, 10_000),
        minHoldSecs: int(k.minHoldSecs, 'trade.caps.minHoldSecs'),
      },
      universe: Array.isArray(t.universe) ? (t.universe as unknown[]).map((x) => { try { return new PublicKey(x as string).toBase58(); } catch { throw new ConfigError('trade.universe: base58 mints'); } }) : null,
    };
  }
  if (has('cranker')) {
    const k = obj(c.cranks, 'cranks (required for the cranker role)');
    need(Array.isArray(k.routes) && (k.routes as unknown[]).every((r) => (CRANK_ROUTES as readonly unknown[]).includes(r)), `cranks.routes: a list of ${CRANK_ROUTES.join(', ')}`);
    cfg.cranks = { routes: k.routes as string[] };
  }
  return cfg;
}

export function loadConfig(path: string): AgentConfig {
  return parseConfig(JSON.parse(readFileSync(path, 'utf8')), dirname(resolve(path)));
}

/** Reads the agent keypair once. The bytes never leave this process. */
export function loadKeypair(path: string): Keypair {
  const raw = JSON.parse(readFileSync(path, 'utf8')) as unknown;
  if (!Array.isArray(raw) || raw.length !== 64) throw new ConfigError('the agent keypair file must be a 64-byte JSON array');
  return Keypair.fromSecretKey(Uint8Array.from(raw as number[]));
}
