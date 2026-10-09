import pg from 'pg';
import { allDdl } from './schema.ts';

export function pool(url = process.env.DATABASE_URL ?? 'postgres://postgres:hookwars@127.0.0.1:5432/hookwars_app'): pg.Pool {
  return new pg.Pool({ connectionString: url, max: 5 });
}

export async function migrate(db: pg.Pool): Promise<number> {
  const ddl = allDdl();
  for (const stmt of ddl) await db.query(stmt);
  return ddl.length;
}
