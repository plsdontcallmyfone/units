import { describe, expect, it } from 'vitest';
import { findBannedWords, type BattleEvent } from '@hookwars/shared';
import { postFor } from './posts.ts';

const cfg = { siteUrl: 'https://site', explorerTx: (s: string) => `https://ex/${s}`, minRaidLamports: 1_000_000_000n };
const ev = (kind: BattleEvent['kind'], amount: string | null): BattleEvent => ({ kind, ts: 1, signature: 'SIG', ordinal: 0, mint: 'MINTaaaaaaaaaaaa', otherMint: 'RIVALbbbbbbbbbbb', actor: null, amount, detail: {} });

describe('posts', () => {
  it('raid above the operator threshold links the join route and the tx', () => {
    const p = postFor(ev('raid', '2000000000'), cfg)!;
    expect(p).toContain('https://site/t/MINTaaaaaaaaaaaa/raid?from=RIVALbbbbbbbbbbb');
    expect(p).toContain('https://ex/SIG');
    expect(findBannedWords(p)).toEqual([]);
  });
  it('small raids and other kinds do not post', () => {
    expect(postFor(ev('raid', '1'), cfg)).toBeNull();
    expect(postFor(ev('forge', null), cfg)).toBeNull();
  });
  it('siege and prize posts', () => {
    expect(postFor(ev('siege', '5000000000'), cfg)).toMatch(/besieged/);
    expect(postFor(ev('prize', '1000000000'), cfg)).toMatch(/Prize paid: 1 SOL/);
  });
});
