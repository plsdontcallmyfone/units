// Changed by Hookwars: new file (explorer v2). The shapes /v1/explorer/* answers with
// (apps/api/src/explorer.ts, packages/sdk/src/hookwars/explore.ts), and small helpers for the pages.
import { PROGRAM_IDS } from '@hookwars/shared';

export type Fact = [string, string];
export interface StoryLine { kind: string; title: string; facts: Fact[]; event?: number }
export interface AccountRef { name: string; pubkey: string; writable: boolean; signer: boolean }
export interface Ix {
  index: number; depth: number; programId: string; program: string | null; name: string | null;
  args: Record<string, unknown> | null; accounts: AccountRef[];
  memo?: { text: string; units: { kind: string; from: string; to: string; thread: string } | null };
  undecoded?: string; inner: Ix[];
}
export interface ExplorerEvent { ordinal: number; program: string; name: string; data: Record<string, unknown>; via: 'cpi' | 'log'; instruction: number }
export interface Explained {
  signature: string | null; slot: number | null; blockTime: number | null; fee: number | null; computeUnits: number | null;
  ok: boolean; error: unknown;
  failure: { programId: string; program: string | null; code: number | null; name: string | null; message: string } | null;
  instructions: Ix[]; events: ExplorerEvent[]; logsTruncated: boolean; story: StoryLine[]; programs: string[];
  logs?: string[];
}
export interface Signature { signature: string; slot: number; blockTime: number | null; ok: boolean; memo: string | null }
export interface AddressPage {
  address: string; exists: boolean; lamports: string | null; owner: string | null; ownerName: string | null;
  executable: boolean; space: number | null; kind: string; program: string | null;
  decoded: { program: string | null; type: string | null; data: Record<string, unknown> | null; error?: string };
  extra: {
    ring?: { filled: number; len: number; spacing: number; lastPriceQ64: string; lastTs: string; twapQ64: string | null; windowSecs: string | null; entries: { ts: string; priceCumulative: string; quoteVolume: string; swapCount: string }[] } | null;
    warChest?: { address: string; lamports: string } | null; warState?: string | null;
  };
  indexed: null | {
    events: { signature: string; ordinal: number; slot: number; program: string; name: string; story: StoryLine | null }[];
    slots?: Record<string, unknown>[]; item?: Record<string, unknown> | null; owner?: Record<string, unknown> | null;
    equippedOn?: { mint: string; slot: number }[]; template?: Record<string, unknown> | null;
  };
  recent: Signature[];
}
export interface ProgramPage {
  address: string; name: string | null; title: string | null; deployed: boolean; instructions: string[]; events: string[];
  indexedCounts: { name: string; n: number; last_slot: string }[] | null; recent: Signature[];
}

const NAMES: Record<string, string> = Object.fromEntries(Object.entries(PROGRAM_IDS).map(([k, v]) => [v, k]));
NAMES['39LXQBGqZtg591jkGnZi9BELQ9hp1ZngbAxu6K1cC29Y'] = 'craft';
NAMES['C4k2QquxzDdgHf74tnvyyWQyGUR8xvhPo1i1gFYb639g'] = 'book';

/** The units program name of an address, or null. */
export const programNameOf = (k: string): string | null => NAMES[k] ?? null;

export const isKey = (v: string): boolean => /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(v);

/** The page an address opens: a program's page for a units program, the account page otherwise. */
export const hrefOf = (k: string): string => (NAMES[k] ? `/program/${k}` : `/address/${k}`);

/** A Q64.64 price as a decimal string (exact integer arithmetic). */
export function q64(q: string | null | undefined, digits = 12): string {
  if (!q || !/^\d+$/.test(q)) return '-';
  const scale = 10n ** BigInt(digits);
  const scaled = (BigInt(q) * scale) >> 64n;
  const frac = (scaled % scale).toString().padStart(digits, '0').replace(/0+$/, '');
  return frac ? `${scaled / scale}.${frac}` : (scaled / scale).toString();
}

/** Decoded account fields flattened to dotted paths, for a two-column table. Long arrays of
 * zeros (reserved space) collapse to one line that says so. */
export function flatten(v: unknown, path = '', out: Fact[] = []): Fact[] {
  if (Array.isArray(v)) {
    if (v.length > 8 && v.every((x) => x === 0 || x === '0')) { out.push([path, `${v.length} zeros`]); return out; }
    if (v.every((x) => typeof x !== 'object' || x === null) && v.length <= 16) { out.push([path, v.map(String).join(', ') || 'empty']); return out; }
    v.forEach((x, i) => flatten(x, `${path}[${i}]`, out));
    return out;
  }
  if (v && typeof v === 'object') {
    for (const [k, x] of Object.entries(v as Record<string, unknown>)) flatten(x, path ? `${path}.${k}` : k, out);
    return out;
  }
  out.push([path, v === null ? 'none' : String(v)]);
  return out;
}
