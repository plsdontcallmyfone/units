// App pass 5: the explorer reads the events pass 4a, pass 4b and secfix3 added. Each one round
// trips through its program's IDL coder and gets its own story line, not the generic fallback.
import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import { coderOf } from './from-idl.ts';
import { decodeEventBody } from './events.ts';
import * as explore from './explore.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));

function roundTrip(program: string, name: string, over: Record<string, unknown> = {}) {
  const c = coderOf(program);
  const disc = c.idl.events!.find((e) => e.name === name)!.discriminator;
  const zero = c.decodeEvent(Buffer.concat([Buffer.from(disc), Buffer.alloc(4000)]))!.data;
  const ev = decodeEventBody(program, c.encodeEvent(name, { ...zero, ...over }));
  expect(ev?.name).toBe(name);
  return explore.storyOf({ program, name, data: explore.plain(ev!.data) as Record<string, unknown> })!;
}

const NEW_EVENTS: [string, string[]][] = [
  ['armory', ['AccessEnforced', 'AccessParamsSet', 'AccessSet', 'AdminActionApplied', 'AdminActionCancelled', 'AdminActionQueued', 'ApprovalRevoked', 'Approved', 'SubmissionSettled', 'TemplateSubmitted']],
  ['book', ['MarketTermsProposed', 'MarketTermsSet', 'Unpayable']],
  ['war', ['BossPoolFunded', 'BossPoolOpened', 'BossPoolSealed', 'BossShareClaimed', 'CoalitionContributed', 'CoalitionDissolved', 'CoalitionFormed', 'CoalitionJoined', 'CoalitionRazed', 'CoalitionSiegeExecuted', 'RivalryOpened', 'RivalrySettled']],
];

describe('new events in the explorer (app pass 5)', () => {
  it('every new event decodes and has its own story', () => {
    for (const [program, names] of NEW_EVENTS) {
      for (const name of names) {
        const st = roundTrip(program, name);
        expect(st.title, `${program}.${name}`).not.toBe(name);
        expect(['armory', 'book', 'war']).toContain(st.kind);
      }
    }
  });
  it('tells access, queue and war facts from the decoded fields', () => {
    const access = roundTrip('armory', 'AccessSet', { item: K(1), holder: K(2), mode: 2, exclusive: true, licenceTerms: { priceLamports: 1_500_000_000n, termSecs: 86_400, per: 0, maxLive: 3 } });
    expect(access.title).toBe('Access set: Licensed');
    expect(access.facts).toEqual(expect.arrayContaining([['Exclusive', 'yes'], ['Licence price', '1.5 SOL'], ['Live licences at most', '3']]));
    const queued = roundTrip('armory', 'AdminActionQueued', { actionHash: Array(32).fill(171), readyAt: 1_700_000_000n });
    expect(queued.facts).toEqual([['Action hash', `0x${'ab'.repeat(32)}`], ['Ready at', '2023-11-14T22:13:20.000Z']]);
    const settled = roundTrip('war', 'RivalrySettled', { mint: K(3), rivalMint: K(4), ours: 10n, theirs: 4n, won: true, spent: 2_000_000_000n, early: false });
    expect(settled.title).toBe('Rivalry won');
    expect(settled.facts).toEqual(expect.arrayContaining([['Spent', '2 SOL']]));
    const unpayable = roundTrip('book', 'Unpayable', { market: K(5), wallet: K(6), amount: 1_000n, paidTo: K(7) });
    expect(unpayable.facts).toEqual(expect.arrayContaining([['Owed to', K(6).toBase58()], ['Paid to', K(7).toBase58()]]));
  });
});
