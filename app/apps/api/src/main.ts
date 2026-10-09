/** `node src/main.ts`. Environment: RPC_URL (default devnet), DATABASE_URL, PORT, HOST. */
import { Connection } from '@solana/web3.js';
import { pool } from '@hookwars/indexer/db.ts';
import { serve } from './server.ts';

const rpcUrl = process.env.RPC_URL ?? 'https://api.devnet.solana.com';
const port = Number(process.env.PORT ?? 9961);
serve({ db: pool(), conn: new Connection(rpcUrl, 'confirmed'), rpcUrl }, port, process.env.HOST ?? '127.0.0.1');
console.log(`hookwars api on ${process.env.HOST ?? '127.0.0.1'}:${port}, rpc ${rpcUrl}`);
