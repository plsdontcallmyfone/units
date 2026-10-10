// Changed by Hookwars: new file (explorer v2). Reads the recorded LiteSVM transactions of
// programs/tests/tests/explorer_fixtures.rs into the explorer's source shape.
import bs58 from 'bs58';
import type { TxSource } from '../explore.ts';

export interface Fixture {
  name: string; ok: boolean; computeUnits: number; accountKeys: string[]; signer: boolean[]; writable: boolean[];
  instructions: { programIdIndex: number; accounts: number[]; dataHex: string }[];
  inner: { index: number; instructions: { programIdIndex: number; accounts: number[]; dataHex: string; stackHeight: number }[] }[];
  logs: string[];
}

const b58 = (hex: string) => bs58.encode(Buffer.from(hex, 'hex'));

export function sourceOf(f: Fixture): TxSource {
  return {
    signature: null as unknown as string, slot: null, blockTime: null, fee: null, computeUnits: f.computeUnits,
    err: f.ok ? null : { recorded: 'failed' },
    accountKeys: f.accountKeys, signer: f.signer, writable: f.writable,
    instructions: f.instructions.map((c) => ({ programIdIndex: c.programIdIndex, accounts: c.accounts, data: b58(c.dataHex) })),
    inner: f.inner.map((g) => ({ index: g.index, instructions: g.instructions.map((c) => ({ programIdIndex: c.programIdIndex, accounts: c.accounts, data: b58(c.dataHex), stackHeight: c.stackHeight })) })),
    logs: f.logs,
  };
}
