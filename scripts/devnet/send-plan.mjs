#!/usr/bin/env node
// Sends scripts/devnet/init-plan.json to devnet, one instruction per transaction, in order.
// DRY RUN BY DEFAULT: prints every step and whether it is already done; sends nothing.
//
//   node scripts/devnet/send-plan.mjs                 # dry run
//   node scripts/devnet/send-plan.mjs --send          # send the steps not yet done
//
// Options: --plan <path> (default scripts/devnet/init-plan.json), --deployer <keypair json>
// (default ~/.config/hookwars/program-keys/deployer-keypair.json), --url <rpc> (default
// https://api.devnet.solana.com or SOLANA_DEVNET_RPC). Never reads ~/.config/solana.
//
// Idempotent and resumable: a step whose `creates` account already exists is skipped.
// Requires app/ dependencies installed (pnpm install in app/) for @solana/web3.js.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(join(root, 'app/packages/sdk/package.json'));
const web3 = require('@solana/web3.js');

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const SEND = args.includes('--send');
const planPath = opt('--plan', join(root, 'scripts/devnet/init-plan.json'));
const deployerPath = opt('--deployer', join(homedir(), '.config/hookwars/program-keys/deployer-keypair.json'));
const url = opt('--url', process.env.SOLANA_DEVNET_RPC ?? 'https://api.devnet.solana.com');
if (url.includes('mainnet')) throw new Error('refusing a mainnet URL: this script is for devnet');

const plan = JSON.parse(readFileSync(planPath, 'utf8'));
const deployer = web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(deployerPath, 'utf8'))));
if (deployer.publicKey.toBase58() !== plan.deployer) {
  throw new Error(`deployer ${deployer.publicKey.toBase58()} is not the plan's ${plan.deployer}`);
}
const conn = new web3.Connection(url, 'confirmed');
console.log(`${SEND ? 'SEND' : 'DRY RUN'} ${plan.steps.length} steps to ${url} as ${plan.deployer}`);
console.log(`deployer balance: ${(await conn.getBalance(deployer.publicKey)) / web3.LAMPORTS_PER_SOL} SOL`);

// One batched read (100 accounts per call) instead of one call per step: the public devnet RPC
// rate-limits bursts of single reads.
const existing = new Set();
const keys = plan.steps.filter((st) => st.creates).map((st) => st.creates);
for (let i = 0; i < keys.length; i += 100) {
  const chunk = keys.slice(i, i + 100);
  const infos = await conn.getMultipleAccountsInfo(chunk.map((k) => new web3.PublicKey(k)));
  infos.forEach((info, j) => { if (info !== null) existing.add(chunk[j]); });
}

let sent = 0;
let skipped = 0;
for (const st of plan.steps) {
  const done = st.creates ? existing.has(st.creates) : false;
  if (done) {
    console.log(`  [${st.step}] skip (exists ${st.creates}): ${st.label}`);
    skipped += 1;
    continue;
  }
  const ix = new web3.TransactionInstruction({
    programId: new web3.PublicKey(st.program),
    keys: st.accounts.map((a) => ({ pubkey: new web3.PublicKey(a.pubkey), isSigner: a.is_signer, isWritable: a.is_writable })),
    data: Buffer.from(st.data_hex, 'hex'),
  });
  if (!SEND) {
    console.log(`  [${st.step}] would send: ${st.label} (${st.accounts.length} accounts, ${st.data_hex.length / 2} data bytes)`);
    continue;
  }
  const tx = new web3.Transaction().add(
    web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
    ix,
  );
  const sig = await web3.sendAndConfirmTransaction(conn, tx, [deployer], { commitment: 'confirmed' });
  console.log(`  [${st.step}] sent ${sig}: ${st.label}`);
  sent += 1;
}
console.log(`done: ${sent} sent, ${skipped} already done`);
