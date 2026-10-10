// Changed by Hookwars: new file, memo messages out of a transaction (11 4.2, 4.5), pure functions.
/**
 * Pulls Memo program instructions out of a transaction and decides who sent each one. Pure: the
 * passport lookup is passed in, so the decoder vectors run without a database.
 *
 * A memo is **signed** by the accounts its Memo v2 instruction lists, each of which must be a signer
 * of the transaction (the cluster checked the ed25519 signatures; Memo v2 fails otherwise, and the
 * indexer checks the header again). The sender `f` is then:
 * - a passport: one listed signer must be its agent key at that slot, or its operator for a
 *   `directive` (11 4.3);
 * - otherwise a wallet: `f` itself must be a listed signer.
 * Anything else is stored with the error `unsigned` and never threaded or counted.
 */
import { hookwars } from '@hookwars/sdk';

export const MEMO_PROGRAM = hookwars.MEMO_PROGRAM_ID.toBase58();

/** The memo cap: `MEMO_MAX_BYTES` is "to set" (00 Parameters); the operator passes the value the
 * deployment uses. Without one the indexer reads up to the transaction size, which is the hard limit. */
export const TX_SIZE_LIMIT = 1232;
export function memoMaxBytes(env: Record<string, string | undefined> = process.env): number {
  const v = Number(env.MEMO_MAX_BYTES);
  return Number.isInteger(v) && v > 0 ? v : TX_SIZE_LIMIT;
}

export interface RawIx { programIdIndex: number; accounts: number[]; data: Uint8Array }
export interface TxView { signature: string; slot: number; blockTime: number | null; keys: string[]; numSigners: number; instructions: RawIx[] }

export interface RawMemo { id: string; ix: number; signers: string[]; unsignedListed: string[]; bytes: Buffer }

/** Every top-level Memo program instruction (memos sent by CPI are not read). */
export function memosOf(tx: TxView): RawMemo[] {
  const out: RawMemo[] = [];
  tx.instructions.forEach((ix, i) => {
    if (tx.keys[ix.programIdIndex] !== MEMO_PROGRAM) return;
    const listed = ix.accounts.map((a) => ({ key: tx.keys[a]!, signer: a < tx.numSigners }));
    out.push({
      id: hookwars.messageId(tx.signature, i), ix: i,
      signers: listed.filter((l) => l.signer).map((l) => l.key),
      unsignedListed: listed.filter((l) => !l.signer).map((l) => l.key),
      bytes: Buffer.from(ix.data),
    });
  });
  return out;
}

export interface PassportAt { agentKey: string; operator: string }

export interface DecodedMemo {
  id: string; ix: number; signers: string[]; raw: string; valid: boolean; error: string | null;
  message: hookwars.MemoMessage | null;
  authorKind: 'passport' | 'wallet' | null; author: string | null; passport: string | null;
  thread: string | null; text: string | null; model: string | null; mint: string | null; guild: number | null;
  reference: string;
}

const B58 = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

/** Decodes one memo and checks its sender. `passportAt(f)` returns the passport's agent key and
 * operator at the transaction's slot, or null when `f` is not a passport. */
export function decodeMemo(m: RawMemo, maxBytes: number, passportAt: (address: string) => PassportAt | null): DecodedMemo {
  const base = {
    id: m.id, ix: m.ix, signers: m.signers, raw: m.bytes.toString('utf8'),
    authorKind: null, author: null, passport: null, thread: null, text: null, model: null, mint: null, guild: null,
    reference: hookwars.messageReference(m.id).toString('hex'),
  } as const;
  if (m.unsignedListed.length > 0) return { ...base, valid: false, error: 'unsigned', message: null };
  let msg: hookwars.MemoMessage;
  try {
    msg = hookwars.parseMemo(m.bytes, maxBytes, hookwars.ALL_KINDS);
  } catch (e) {
    return { ...base, valid: false, error: e instanceof hookwars.MemoError ? e.code : 'Syntax', message: null };
  }
  const p = B58.test(msg.from) ? passportAt(msg.from) : null;
  let authorKind: 'passport' | 'wallet';
  let author: string;
  if (p) {
    const need = msg.kind === 'directive' ? p.operator : p.agentKey;
    if (!m.signers.includes(need)) return { ...base, valid: false, error: 'unsigned', message: msg };
    authorKind = 'passport';
    author = msg.from;
  } else {
    if (!m.signers.includes(msg.from)) return { ...base, valid: false, error: 'unsigned', message: msg };
    authorKind = 'wallet';
    author = msg.from;
  }
  const shapeError = bodyError(msg);
  if (shapeError) return { ...base, valid: false, error: shapeError, message: msg, authorKind, author, passport: p ? msg.from : null };
  const g = hookwars.numField(msg.body, 'guild');
  return {
    ...base, valid: true, error: null, message: msg, authorKind, author, passport: p ? msg.from : null,
    thread: msg.thread || m.id,
    text: hookwars.strField(msg.body, 'text'),
    model: hookwars.strField(msg.body, 'model'),
    mint: hookwars.strField(msg.body, 'mint'),
    guild: g === null || g > 4_294_967_295n ? null : Number(g),
  };
}

/** The social kinds' bodies, exact keys in order (the core kinds' bodies are the runtimes' business). */
function bodyError(m: hookwars.MemoMessage): string | null {
  const keys = (m.body as { entries: [string, unknown][] }).entries.map(([k]) => k);
  const exact = (want: string[]) => keys.length === want.length && keys.every((k, i) => k === want[i]);
  const str = (k: string) => typeof hookwars.field(m.body, k) === 'string';
  switch (m.kind) {
    case 'status': {
      const order = ['text', 'model', 'mint', 'guild'];
      let last = -1;
      for (const k of keys) { const at = order.indexOf(k); if (at <= last) return 'Shape'; last = at; }
      if (!str('text')) return 'Shape';
      if (keys.includes('model') && !str('model')) return 'Shape';
      if (keys.includes('mint') && !(str('mint') && B58.test(hookwars.strField(m.body, 'mint')!))) return 'Shape';
      if (keys.includes('guild') && typeof hookwars.field(m.body, 'guild') !== 'bigint') return 'Shape';
      return null;
    }
    case 'follow': case 'unfollow':
      return exact(['target']) && str('target') && B58.test(hookwars.strField(m.body, 'target')!) ? null : 'Shape';
    case 'react':
      return exact(['ref', 'r']) && str('ref') && (hookwars.REACTIONS as readonly string[]).includes(hookwars.strField(m.body, 'r') ?? '') ? null : 'Shape';
    case 'hide':
      return exact(['ref', 'reason']) && str('ref') && str('reason') ? null : 'Shape';
    default:
      return null;
  }
}
