/**
 * The IDL builders against `vectors/hookwars-math.json` "instructions": each case is the same call
 * rendered by the programs' own Rust clients (`hookwars_war::client`, `bordrless_token::client`) in
 * `programs/tests/tests/app_vectors.rs`. Program id, every account (key, signer, writable) and the
 * data must be equal.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, TransactionInstruction, type AccountMeta } from '@solana/web3.js';
import raw from '../../../shared/vectors/hookwars-math.json' with { type: 'json' };
import {
  accrueTreatyTime, cancelRoll, claimBounty, claimQuest, counterStrike, finalizeSeason, initWar, openSeason, raze, recordFunding,
  returnCaptured, roll, shareTreatyInflow, siege, splitProtocolFees, submitCandidate, touch,
} from './instructions.ts';

const V = raw as unknown as { instructions: { name: string; programId: string; keys: [string, boolean, boolean][]; data: string }[] };
const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const m = (n: number, w: boolean): AccountMeta => ({ pubkey: K(n), isSigner: false, isWritable: w });
const inner = new TransactionInstruction({ programId: K(200), keys: [m(201, true), m(202, false)], data: Buffer.alloc(0) });
const orders = { item: K(30), template: K(31) };

const BUILT: Record<string, () => TransactionInstruction> = {
  initWar: () => initWar(K(1), K(2)),
  recordFunding: () => recordFunding(K(2)),
  claimBounty: () => claimBounty(K(3), K(2), orders, 1, [m(40, true), m(41, false)], []),
  claimQuestRaid: () => claimQuest(K(3), K(2), 4, 1, 9, 1, [m(40, true)]),
  claimQuestForge: () => claimQuest(K(3), K(2), 4, 2, 9, 1, []),
  roll: () => roll(K(3), K(2), 7n, 1, { program: K(50), account: K(51) }, [m(40, true)]),
  cancelRoll: () => cancelRoll(K(3), K(2), 7n),
  openSeason0: () => openSeason(0),
  openSeason3: () => openSeason(3),
  submitCandidateLedger: () => submitCandidate(K(3), 5, K(2), true),
  submitCandidateNoLedger: () => submitCandidate(K(3), 5, K(2), false),
  finalizeSeason: () => finalizeSeason(5),
  splitNoWinner: () => splitProtocolFees(K(3), K(60), null),
  splitWinner: () => splitProtocolFees(K(3), K(60), { mint: K(2), season: 4 }, [inner]),
  accrueTreatyTime: () => accrueTreatyTime(K(2), 4, [[K(70), K(71)], [K(72), K(73)]]),
  siege: () => siege(K(3), K(2), orders, K(80), K(81), true, K(82), [m(83, true), m(84, false)], [inner]),
  siegeNoWar: () => siege(K(3), K(2), orders, K(80), K(81), false, null, [], []),
  counterStrike: () => counterStrike(K(3), K(2), orders, K(81), null, [m(83, true)], [m(84, true), m(85, false)], [inner]),
  raze: () => raze(K(3), K(2), orders, K(80), K(81), [m(83, true)], []),
  returnCaptured: () => returnCaptured(K(3), K(2), K(80), { item: K(90), template: K(91) }, [m(83, true)], []),
  shareTreatyInflow: () => shareTreatyInflow(K(3), K(2), [inner]),
  touch: () => touch(K(3), K(2), K(4), K(5), 1, Buffer.from([1, 2, 3]), [m(40, true)]),
};

describe('IDL builders equal the Rust clients', () => {
  it('covers every rendered case', () => {
    expect(V.instructions.map((c) => c.name).sort()).toEqual(Object.keys(BUILT).sort());
  });
  for (const c of V.instructions) {
    it(c.name, () => {
      const ix = BUILT[c.name]!();
      expect(ix.programId.toBase58()).toBe(c.programId);
      expect(ix.keys.map((k) => [k.pubkey.toBase58(), k.isSigner, k.isWritable])).toEqual(c.keys);
      expect(ix.data.toString('hex')).toBe(c.data);
    });
  }
});
