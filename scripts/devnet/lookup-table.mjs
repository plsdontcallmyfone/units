#!/usr/bin/env node
// Creates the protocol address lookup table on devnet with exactly the addresses the app expects
// (PROTOCOL_LOOKUP_TABLE_ADDRESSES in app/packages/shared/src/programs.ts, in order), then prints
// the table address for the app's PROTOCOL_LOOKUP_TABLE setting.
// DRY RUN BY DEFAULT.
//
//   node --experimental-strip-types scripts/devnet/lookup-table.mjs           # dry run
//   node --experimental-strip-types scripts/devnet/lookup-table.mjs --send    # create and extend
//
// Options as send-plan.mjs (--deployer, --url). The deployer is the table's authority. A table
// becomes usable one slot after its last extension.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(join(root, 'app/packages/sdk/package.json'));
const web3 = require('@solana/web3.js');
const { PROTOCOL_LOOKUP_TABLE_ADDRESSES } = await import(
  pathToFileURL(join(root, 'app/packages/shared/src/programs.ts')).href
);

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const SEND = args.includes('--send');
const deployerPath = opt('--deployer', join(homedir(), '.config/hookwars/program-keys/deployer-keypair.json'));
const url = opt('--url', process.env.SOLANA_DEVNET_RPC ?? 'https://api.devnet.solana.com');
if (url.includes('mainnet')) throw new Error('refusing a mainnet URL: this script is for devnet');

const deployer = web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(deployerPath, 'utf8'))));
const addresses = PROTOCOL_LOOKUP_TABLE_ADDRESSES.map((a) => new web3.PublicKey(a));
console.log(`${SEND ? 'SEND' : 'DRY RUN'}: protocol lookup table with ${addresses.length} addresses, authority ${deployer.publicKey.toBase58()}`);
if (!SEND) {
  addresses.forEach((a, i) => console.log(`  ${i}: ${a.toBase58()}`));
  process.exit(0);
}
const conn = new web3.Connection(url, 'confirmed');
const slot = await conn.getSlot('finalized');
const [createIx, table] = web3.AddressLookupTableProgram.createLookupTable({
  authority: deployer.publicKey,
  payer: deployer.publicKey,
  recentSlot: slot,
});
await web3.sendAndConfirmTransaction(conn, new web3.Transaction().add(createIx), [deployer]);
console.log(`created ${table.toBase58()}`);
for (let i = 0; i < addresses.length; i += 20) {
  const ix = web3.AddressLookupTableProgram.extendLookupTable({
    lookupTable: table,
    authority: deployer.publicKey,
    payer: deployer.publicKey,
    addresses: addresses.slice(i, i + 20),
  });
  const sig = await web3.sendAndConfirmTransaction(conn, new web3.Transaction().add(ix), [deployer]);
  console.log(`  extended ${i}..${Math.min(i + 20, addresses.length) - 1}: ${sig}`);
}
console.log(`PROTOCOL_LOOKUP_TABLE=${table.toBase58()}`);
