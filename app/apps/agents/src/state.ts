// The runtime's own small state per agent: the trading book, rate-limit windows, the facts cursor
// and the directive it last followed. Plain JSON on disk (bigints as strings), written atomically.
import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Book } from './policy.ts';

export interface AgentState {
  book: Book | null;
  /** Unix times of memos sent (for `memo.maxPerHour` / `maxPerDay`). */
  memoTimes: number[];
  /** Unix times of items created (for `author.maxItemsPerDay`). */
  itemTimes: number[];
  factsCursor: string | null;
  /** Facts seen and not yet reported in a status memo. */
  pendingFacts: { signature: string; text: string }[];
  directiveSeq: number | null;
  /** The directive sequence whose `clearHalt` was already applied. */
  clearedHaltSeq: number | null;
}

export function emptyState(): AgentState {
  return { book: null, memoTimes: [], itemTimes: [], factsCursor: null, pendingFacts: [], directiveSeq: null, clearedHaltSeq: null };
}

export interface StateStore { load(): AgentState; save(s: AgentState): void }

const replacer = (_k: string, v: unknown) => (typeof v === 'bigint' ? { $big: v.toString() } : v);
const reviver = (_k: string, v: unknown) => (v && typeof v === 'object' && '$big' in (v as object) ? BigInt((v as { $big: string }).$big) : v);

export function serialize(s: AgentState): string { return JSON.stringify(s, replacer, 1); }
export function deserialize(t: string): AgentState { return { ...emptyState(), ...(JSON.parse(t, reviver) as AgentState) }; }

export function fileStore(dir: string): StateStore {
  const path = join(dir, 'state.json');
  return {
    load() { try { return deserialize(readFileSync(path, 'utf8')); } catch { return emptyState(); } },
    save(s) { mkdirSync(dir, { recursive: true }); writeFileSync(path + '.tmp', serialize(s)); renameSync(path + '.tmp', path); },
  };
}

export function memoryStore(initial: AgentState = emptyState()): StateStore & { current: AgentState } {
  const st = { current: deserialize(serialize(initial)) };
  return Object.assign(st, { load: () => deserialize(serialize(st.current)), save: (s: AgentState) => { st.current = deserialize(serialize(s)); } });
}

/** Keeps only times inside the last `windowSecs`. */
export function within(times: number[], now: number, windowSecs: number): number[] { return times.filter((t) => now - t < windowSecs); }
