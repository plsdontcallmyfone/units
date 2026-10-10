// Prints what the explorer decodes from each recorded fixture (a review aid; the test asserts it).
import { readdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { explainTransaction } from '../src/hookwars/explore.ts';
import { sourceOf } from '../src/hookwars/fixtures/load.ts';
const dir = join(dirname(fileURLToPath(import.meta.url)), '..', 'src', 'hookwars', 'fixtures', 'explorer');
for (const f of readdirSync(dir).filter((x) => x.endsWith('.json')).sort()) {
  const fx = JSON.parse(readFileSync(join(dir, f), 'utf8'));
  const t = explainTransaction(sourceOf(fx));
  console.log(`== ${fx.name} ok=${t.ok} failure=${JSON.stringify(t.failure)}`);
  const walk = (xs, d) => xs.forEach((x) => { console.log(`${'  '.repeat(d)}${x.program ?? x.programId}.${x.name ?? '?'}${x.undecoded ? ` [${x.undecoded}]` : ''}`); walk(x.inner, d + 1); });
  walk(t.instructions, 1);
  for (const e of t.events) console.log(`  event ${e.ordinal} ${e.program}.${e.name} via ${e.via} ix ${e.instruction} ${JSON.stringify(e.data).slice(0, 300)}`);
  for (const s of t.story) console.log(`  story ${s.kind}: ${s.title} | ${s.facts.map((x) => x.join('=')).join('; ').slice(0, 300)}`);
}
