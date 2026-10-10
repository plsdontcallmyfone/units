// Changed by Hookwars: new file (launch page). The page's checks name the program errors they mirror.
import { describe, expect, it } from 'vitest';
import { budget, compositeProblems, hostKind, type SlotDraft } from './checks';

const slot = (p: Partial<SlotDraft>): SlotDraft => ({ kind: 'pool', rule: 'vote', maxCutBps: 0, noticeSecs: 0, templateId: null, launchItem: '', targets: [], ...p });

describe('budget', () => {
  it('counts the kit as a slot and as 32 bytes of holder memory', () => {
    const b = budget([slot({}), slot({}), slot({})], true);
    expect(b.slots).toBe(4);
    expect(b.bytesFree).toBe(32);
    expect(b.problems).toEqual([]);
    expect(budget([slot({}), slot({}), slot({}), slot({})], true).problems.join(' ')).toMatch(/TooManySlots/);
  });
  it('limits token-side cutting slots and holder memory', () => {
    const fee = () => slot({ kind: 'fee', maxCutBps: 100, templateId: 7 });
    expect(budget([fee(), fee(), fee()], true).problems.join(' ')).toMatch(/TooManyCuttingSlots/);
    expect(budget([slot({ templateId: 1 }), slot({ templateId: 1 }), slot({ templateId: 1 })], true).problems.join(' ')).toMatch(/HookDataOverflow/);
    expect(budget([slot({ kind: 'fee', templateId: 1 })], false).problems.join(' ')).toMatch(/KindMismatch/);
  });
});

describe('composites', () => {
  it('mirrors the armory: hosts, conflicts, touch, targets', () => {
    expect(hostKind(['fee', 'reward'])).toBe('reward');
    expect(hostKind(['relation', 'pool'])).toBeNull();
    expect(compositeProblems([]).problems.join(' ')).toMatch(/TooManyModules/);
    expect(compositeProblems([{ templateId: 9, params: [], targetStart: 0, targetCount: 0 }]).problems.join(' ')).toMatch(/NotComposable/);
    expect(compositeProblems([{ templateId: 2, params: [], targetStart: 0, targetCount: 1 }, { templateId: 40, params: [], targetStart: 1, targetCount: 0 }]).problems.join(' ')).toMatch(/ModuleConflict/);
    expect(compositeProblems([{ templateId: 7, params: [], targetStart: 0, targetCount: 0 }, { templateId: 2, params: [], targetStart: 0, targetCount: 0 }], 'fee').problems.join(' ')).toMatch(/KindMismatch/);
    expect(compositeProblems([{ templateId: 2, params: [], targetStart: 0, targetCount: 2 }, { templateId: 7, params: [], targetStart: 1, targetCount: 1 }]).problems.join(' ')).toMatch(/BadTargets/);
  });
});
