import { describe, expect, it } from 'vitest';
import { hookwars } from '@hookwars/sdk';
import { allDdl, eventTable, eventTablesDdl } from './schema.ts';
import { CURSOR_PROGRAMS } from './indexer.ts';

describe('schema', () => {
  it('one typed table per Hookwars event, keyed by (signature, ordinal)', () => {
    const n = Object.values(hookwars.EVENT_SPECS).reduce((a, s) => a + s.length, 0);
    const ddl = eventTablesDdl();
    expect(ddl).toHaveLength(n);
    for (const s of ddl) expect(s).toContain('primary key (signature, ordinal)');
    expect(eventTable('items', 'RaidMarked')).toBe('ev_items_raid_marked');
    expect(eventTable('war', 'WarChestCreated')).toBe('ev_war_war_chest_created');
  });
  it('views reference only generated tables', () => {
    const tables = new Set(eventTablesDdl().map((s) => /create table if not exists (\w+)/.exec(s)![1]));
    for (const v of allDdl().filter((s) => s.startsWith('create or replace view'))) {
      for (const m of v.matchAll(/from (ev_\w+)/g)) expect(tables.has(m[1]!)).toBe(true);
    }
  });
  it('nine cursors (06 2.1)', () => {
    expect(CURSOR_PROGRAMS.map((c) => c.program)).toEqual(['token', 'swap', 'launch', 'kit', 'bridge', 'companion', 'armory', 'items', 'war']);
  });
});
