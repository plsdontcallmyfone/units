// Changed by Hookwars: log events of items and the launchpad, attributed to their program.
/**
 * Every event of one transaction, in execution order (06 2.1): self-CPI events of every program
 * (Hookwars schemas first, then the upstream IDL coders) and `hookwars_items` log events, numbered
 * in one sequence. The ordinal of a log event is placed after the self-CPI events emitted before
 * its log line; when logs are truncated the item events are hints and the caller re-reads accounts.
 */
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { decodeEventPayload, flatten, programNameOf as upstreamProgramNameOf, type RawInnerInstruction } from '../events.ts';
import { eventAuthority } from '../addresses.ts';
import { decodeCpiEvent, programLogEvents, logsTruncated, programNameOf, EVENT_IX_TAG, PROGRAM_OF } from './events.ts';

export interface TxEvent { ordinal: number; program: string; programId: string; name: string; data: Record<string, unknown>; via: 'cpi' | 'log' }

export interface TxInput {
  accountKeys: string[];
  inner: RawInnerInstruction[];
  logs: readonly string[];
}

export function decodeTransactionEvents(tx: TxInput): { events: TxEvent[]; truncated: boolean } {
  const cpi: TxEvent[] = [];
  for (const ix of tx.inner) {
    const programId = tx.accountKeys[ix.programIdIndex];
    if (!programId) continue;
    const hw = programNameOf(programId);
    const up = upstreamProgramNameOf(programId);
    if (!hw && !up) continue;
    const bytes = Buffer.from(bs58.decode(ix.data));
    if (bytes.length < 16 || !bytes.subarray(0, 8).equals(EVENT_IX_TAG)) continue;
    if (ix.accounts.length !== 1 || tx.accountKeys[ix.accounts[0]!] !== eventAuthority(new PublicKey(programId)).toBase58()) continue;
    let ev: { name: string; data: Record<string, unknown> } | null = null;
    if (hw) {
      const d = decodeCpiEvent(hw, bytes);
      if (d) ev = { name: d.name, data: flatten(d.data) as Record<string, unknown> };
    }
    if (!ev && up) {
      const d = decodeEventPayload(up, bytes.subarray(8));
      if (d) ev = { name: d.name.charAt(0).toUpperCase() + d.name.slice(1), data: d.data };
    }
    if (ev) cpi.push({ ordinal: 0, program: hw ?? up!, programId, name: ev.name, data: ev.data, via: 'cpi' });
  }
  const logs = programLogEvents(tx.logs).map((e) => ({ ordinal: 0, program: e.program, programId: PROGRAM_OF[e.program]?.toBase58() ?? '', name: e.name, data: flatten(e.data) as Record<string, unknown>, via: 'log' as const }));
  const events = [...cpi, ...logs].map((e, i) => ({ ...e, ordinal: i }));
  return { events, truncated: logsTruncated(tx.logs) };
}
