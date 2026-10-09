// Changed by Hookwars: the action form's body builder (lib/action-body.ts).
import { describe, expect, it } from 'vitest';
import { bodyOf, solToLamports, type Field } from './action-body';

describe('action form bodies', () => {
  it('converts SOL to lamports exactly', () => {
    expect(solToLamports('1')).toBe('1000000000');
    expect(solToLamports('0.000000001')).toBe('1');
    expect(solToLamports('2.5')).toBe('2500000000');
    expect(solToLamports('1.0000000001')).toBeNull();
    expect(solToLamports('-1')).toBeNull();
  });
  it('builds typed, nested bodies over fixed values', () => {
    const fields: Field[] = [
      { name: 'kind.to', label: 'To', kind: 'key' }, { name: 'kind.lamports', label: 'Amount', kind: 'sol' },
      { name: 'slot', label: 'Slot', kind: 'int' }, { name: 'support', label: 'For', kind: 'bool' },
      { name: 'targets', label: 'Targets', kind: 'keys', optional: true }, { name: 'ids', label: 'Ids', kind: 'ints' },
    ];
    const r = bodyOf(fields, { 'kind.to': 'Abc', 'kind.lamports': '0.5', slot: '2', support: 'true', ids: '1, 9' }, { guildId: 3, kind: { name: 'SpendSol' }, targets: [] });
    expect(r).toEqual({ body: { guildId: 3, kind: { name: 'SpendSol', to: 'Abc', lamports: '500000000' }, slot: 2, support: true, targets: [], ids: [1, 9] } });
  });
  it('names the first field that does not parse, and requires non-optional fields', () => {
    expect(bodyOf([{ name: 'slot', label: 'Slot', kind: 'int' }], { slot: 'x' }, {})).toEqual({ error: 'Slot must be a whole number.' });
    expect(bodyOf([{ name: 'name', label: 'Name', kind: 'text' }], {}, {})).toEqual({ error: 'Name is required.' });
    expect(bodyOf([{ name: 'bad', label: 'Ids', kind: 'ints' }], { bad: '1,a' }, {})).toEqual({ error: 'Ids must be whole numbers separated by commas.' });
  });
  it('does not mutate the fixed values', () => {
    const fixed = { kind: { name: 'SpendSol' } };
    bodyOf([{ name: 'kind.to', label: 'To', kind: 'key' }], { 'kind.to': 'X' }, fixed);
    expect(fixed).toEqual({ kind: { name: 'SpendSol' } });
  });
});
