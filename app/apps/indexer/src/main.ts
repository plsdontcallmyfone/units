// Changed by Hookwars: each pass also walks the social authors' memos.
/**
 * `node src/main.ts migrate | once | run`. Environment: RPC_URL (default devnet), DATABASE_URL,
 * INDEXER_INTERVAL_MS (run mode).
 */
import { Connection } from '@solana/web3.js';
import { migrate, pool } from './db.ts';
import { indexOnce, refreshProgramInfo } from './indexer.ts';
import { indexMemoAuthors } from './social.ts';

const rpc = process.env.RPC_URL ?? 'https://api.devnet.solana.com';
const mode = process.argv[2] ?? 'once';
const db = pool();
const conn = new Connection(rpc, 'confirmed');

async function pass(): Promise<void> {
  await refreshProgramInfo(conn, db);
  const stats = await indexOnce(conn, db);
  const social = await indexMemoAuthors(conn, db);
  console.log(JSON.stringify({ at: new Date().toISOString(), rpc, stats, social }));
}

try {
  const n = await migrate(db);
  if (mode === 'migrate') console.log(`migrated: ${n} statements`);
  else if (mode === 'once') await pass();
  else {
    const every = Number(process.env.INDEXER_INTERVAL_MS ?? 10_000);
    for (;;) {
      try { await pass(); } catch (e) { console.error(e); }
      await new Promise((r) => setTimeout(r, every));
    }
  }
} finally {
  if (mode !== 'run') await db.end();
}
