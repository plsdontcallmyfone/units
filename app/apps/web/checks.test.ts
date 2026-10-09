/** 06 section 8: no file contains U+2014 and no style names a monospace face; page copy passes the
 * banned-words check. The computed-style check runs in the screenshot pass (app/screenshots). */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { findBannedWords } from '@hookwars/shared';

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    if (f === 'node_modules' || f === '.next') return [];
    return statSync(p).isDirectory() ? files(p) : /\.(tsx?|css)$/.test(f) ? [p] : [];
  });
}

describe('site copy and styles', () => {
  const all = [...files(join(__dirname, 'app')), ...files(join(__dirname, 'components'))];
  it('has no em dash', () => { for (const f of all) expect(readFileSync(f, 'utf8').includes('—'), f).toBe(false); });
  it('names no monospace face', () => { for (const f of all) expect(/monospace|ui-monospace|font-mono|Menlo|Courier/.test(readFileSync(f, 'utf8')), f).toBe(false); });
  it('says nothing banned in visible strings', () => {
    for (const f of all.filter((x) => x.endsWith('.tsx'))) {
      const strings = [...readFileSync(f, 'utf8').matchAll(/>([^<>{}]{4,})</g)].map((m) => m[1]!);
      for (const s of strings) expect(findBannedWords(s), `${f}: ${s}`).toEqual([]);
    }
  });
});
