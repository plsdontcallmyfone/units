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

// `--demo <file>`: the web's MOCK_DATA explorer pages, made from these recorded transactions as the
// explorer decodes them (each gets a stand-in signature: LiteSVM records none). Shown under "Demo data".
const at = process.argv.indexOf('--demo');
if (at > 0) {
  const { createHash } = await import('node:crypto');
  const { writeFileSync } = await import('node:fs');
  const bs58 = (await import('bs58')).default;
  const { EXPLORER_IDLS, PROGRAM_TITLES, storyOf } = await import('../src/hookwars/explore.ts');
  const txs = {}; const byAddress = {};
  for (const f of readdirSync(dir).filter((x) => x.endsWith('.json')).sort()) {
    const fx = JSON.parse(readFileSync(join(dir, f), 'utf8'));
    const h = createHash('sha512').update(`units demo ${fx.name}`).digest();
    const sig = bs58.encode(h);
    const t = explainTransaction({ ...sourceOf(fx), signature: sig, slot: 400_000_000 + Object.keys(txs).length, blockTime: 1_800_000_000 + Object.keys(txs).length * 60, fee: 5000 * fx.signer.filter(Boolean).length });
    txs[sig] = { ...t, logs: fx.logs, demo: fx.name };
    for (const e of t.events) for (const k of ['mint', 'item', 'owner', 'trader', 'pool']) {
      const v = e.data[k];
      if (typeof v === 'string') (byAddress[v] ??= []).push({ signature: sig, ordinal: e.ordinal, slot: t.slot, program: e.program, name: e.name, story: storyOf(e) });
    }
  }
  const programs = Object.entries(EXPLORER_IDLS).map(([name, idl]) => ({ name, title: PROGRAM_TITLES[name] ?? name, address: idl.address, instructions: idl.instructions.map((x) => x.name), events: (idl.events ?? []).map((x) => x.name) }));
  writeFileSync(process.argv[at + 1], JSON.stringify({ programs, txs, byAddress }));
  console.log(`demo: ${Object.keys(txs).length} transactions, ${Object.keys(byAddress).length} addresses`);
}
