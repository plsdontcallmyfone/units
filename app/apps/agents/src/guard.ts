// Security review 3 (M-10, L-9): what the runtime refuses before it signs or fetches. A prepare
// route's answer is not trusted: every instruction must call a program the route is allowed to
// reach at the top level, the token program may only open holdings for the agent or its vault, and
// the agent key never signs a swap, a sale or a bridge transfer of its own funds. Rules documents
// come over https from public addresses only.
import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';
import { PublicKey, type TransactionInstruction } from '@solana/web3.js';
import { hookwars, token } from '@hookwars/sdk';

/** Top-level programs each prepare route may return (what the API builds for it, nothing more). */
export const ROUTE_PROGRAMS: Readonly<Record<string, readonly PublicKey[]>> = {
  'buy/prepare': [hookwars.SWAP_ID, hookwars.TOKEN_ID],
  'sell/prepare': [hookwars.SWAP_ID, hookwars.TOKEN_ID, hookwars.BRIDGE_ID],
  'settle/prepare': [hookwars.ITEMS_ID, hookwars.TOKEN_ID],
  'seasons/open/prepare': [hookwars.WAR_ID],
  'seasons/finalize/prepare': [hookwars.WAR_ID],
  'prize/split/prepare': [hookwars.WAR_ID],
  'proposals/finalize/prepare': [hookwars.ARMORY_ID],
  'votes/close/prepare': [hookwars.ARMORY_ID],
  'proposals/close/prepare': [hookwars.ARMORY_ID],
  'agents/bonds/resolve/prepare': [hookwars.AGENTS_ID],
  'war/siege/prepare': [hookwars.WAR_ID, hookwars.TOKEN_ID],
  'war/counter-strike/prepare': [hookwars.WAR_ID, hookwars.TOKEN_ID],
  'war/raze/prepare': [hookwars.WAR_ID, hookwars.TOKEN_ID],
  'book/crank/prepare': [hookwars.BOOK_ID, hookwars.TOKEN_ID],
  'loyalty/reslot/prepare': [hookwars.ITEMS_ID],
};

/** Programs where a signature of the agent key itself could move the agent key's own money. */
const MONEY_PROGRAMS = [hookwars.SWAP_ID, hookwars.LAUNCH_ID, hookwars.MARKET_ID, hookwars.BRIDGE_ID, hookwars.BOOK_ID];

const CREATE_HOLDING = token.createHolding(PublicKey.default, PublicKey.default, PublicKey.default).data.subarray(0, 8);

const signs = (ix: TransactionInstruction, key: PublicKey) => ix.keys.some((k) => k.isSigner && k.pubkey.equals(key));

/**
 * Problems with a prepared transaction for `route`, before any wrapping: an empty list means it
 * may be signed. `owner` is whose funds the route acts on (the agent key for cranks, the vault for
 * trades).
 */
export function preparedProblems(route: string, ixs: readonly TransactionInstruction[], agentKey: PublicKey, vault: PublicKey): string[] {
  const allowed = ROUTE_PROGRAMS[route];
  if (!allowed) return [`route ${route} has no program allowlist in the runtime`];
  const out: string[] = [];
  ixs.forEach((ix, i) => {
    const p = ix.programId;
    if (!allowed.some((a) => a.equals(p))) { out.push(`instruction ${i} calls ${p.toBase58()}, which ${route} never needs`); return; }
    const byAgent = signs(ix, agentKey);
    const byVault = signs(ix, vault);
    if (p.equals(hookwars.TOKEN_ID) && (byAgent || byVault) && !ix.data.subarray(0, 8).equals(CREATE_HOLDING)) {
      out.push(`instruction ${i} is a token instruction other than create_holding signed by the agent or its vault`);
    }
    if (byAgent && MONEY_PROGRAMS.some((m) => m.equals(p))) out.push(`instruction ${i} has the agent key sign a ${p.toBase58()} instruction that could spend its funds`);
  });
  return out;
}

const LOOPBACK = new Set(['localhost', '127.0.0.1', '::1', '[::1]']);

/** The API must be https (plain http only on this machine), so a prepared transaction cannot be swapped in transit. */
export function checkApiUrl(raw: string): string {
  let u: URL;
  try { u = new URL(raw); } catch { throw new Error('UNITS_API_URL is not a URL'); }
  if (u.protocol === 'https:') return raw;
  if (u.protocol === 'http:' && LOOPBACK.has(u.hostname)) return raw;
  throw new Error('UNITS_API_URL must be https (plain http is accepted only for localhost)');
}

/** Private, loopback, link-local, unique-local, CGNAT and unspecified addresses (IPv4 and IPv6). */
export function privateAddress(ip: string): boolean {
  const v = isIP(ip);
  if (v === 4) {
    const [a, b] = ip.split('.').map(Number) as [number, number];
    return a === 10 || a === 127 || a === 0 || (a === 169 && b === 254) || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168) || (a === 100 && b >= 64 && b <= 127) || a >= 224;
  }
  if (v === 6) {
    const s = ip.toLowerCase();
    if (s === '::' || s === '::1') return true;
    const mapped = s.match(/^::ffff:(\d+\.\d+\.\d+\.\d+)$/);
    if (mapped) return privateAddress(mapped[1]!);
    return /^f[cd]/.test(s) || /^fe[89ab]/.test(s) || /^ff/.test(s);
  }
  return true;
}

export type Resolver = (host: string) => Promise<string[]>;
const dnsResolver: Resolver = async (host) => (await lookup(host, { all: true, verbatim: true })).map((a) => a.address);

/** A rules URI the runtime may fetch: https, and every address its host resolves to is public. */
export async function checkRulesUri(raw: string, resolve: Resolver = dnsResolver): Promise<URL> {
  let u: URL;
  try { u = new URL(raw); } catch { throw new Error('rules_uri is not a URL'); }
  if (u.protocol !== 'https:') throw new Error('rules_uri must be https');
  if (u.username || u.password) throw new Error('rules_uri may not carry credentials');
  const host = u.hostname.replace(/^\[|\]$/g, '');
  const addrs = isIP(host) ? [host] : await resolve(host);
  if (!addrs.length || addrs.some(privateAddress)) throw new Error('rules_uri resolves to a private or local address');
  return u;
}
