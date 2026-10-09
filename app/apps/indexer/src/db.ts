// Changed by Hookwars: DATABASE_URL required, no default credentials (app audit A-9).
import pg from 'pg';
import { allDdl } from './schema.ts';

/** The database URL, required: no default credentials live in code (app audit A-9). Give the API a
 * read-only role and the indexer a writing one. */
export function databaseUrl(env: Record<string, string | undefined> = process.env): string {
  const url = env.DATABASE_URL;
  if (!url) throw new Error('DATABASE_URL is not set; the indexer and the API need it (no default credentials).');
  return url;
}

export function pool(url = databaseUrl()): pg.Pool {
  return new pg.Pool({ connectionString: url, max: 5 });
}

export async function migrate(db: pg.Pool): Promise<number> {
  const ddl = allDdl();
  for (const stmt of ddl) await db.query(stmt);
  return ddl.length;
}
