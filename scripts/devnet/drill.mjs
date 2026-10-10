#!/usr/bin/env node
// The devnet drill driver (docs/DEVNET.md section 5): one prepare route per call, through the
// running API, signed by a drill wallet and sent to devnet. Every landed signature is appended to
// scripts/devnet/drill-log.jsonl.
//
//   node scripts/devnet/drill.mjs wallets trader1 trader2 ...      # create drill wallets (once)
//   node scripts/devnet/drill.mjs fund <wallet> <sol>              # deployer pays a drill wallet
//   node --experimental-strip-types scripts/devnet/drill.mjs run <route> '<json body>' --as <wallet>
//        [--save <name>[=<path>]] [--item <name>] [--dry]
//   node scripts/devnet/drill.mjs wrap <wallet> <sol>                # bridge wrap_sol
//   node scripts/devnet/drill.mjs show                             # wallets, balances, saved state
//
// Body strings are substituted before the call: "$w:<wallet>" is that wallet's address,
// "$s:<name>" a value saved earlier, and "$new:<name>" a fresh keypair (saved under that name)
// whose address goes in the body and which signs when the API lists it as an extra signer
// ("mint" or "config"). --save <name>=<path> saves a value from the API response (dot path) or,
// with no path, the address of the "$new:" key used for the mint.
//
// Options: --api (default API_URL or http://127.0.0.1:9961), --url (devnet RPC), --keys (drill
// wallet folder, default <repo>/keys/drill), --deployer (default <repo>/keys/devnet/deployer-keypair.json).
// Never reads ~/.config/solana and never prints a secret key.
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(join(root, 'app/packages/sdk/package.json'));
const web3 = require('@solana/web3.js');

const argv = process.argv.slice(2);
const opt = (name, fallback) => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : fallback; };
const flag = (name) => argv.includes(name);
const API = opt('--api', process.env.API_URL ?? 'http://127.0.0.1:9961');
const URL_ = opt('--url', process.env.SOLANA_DEVNET_RPC ?? 'https://api.devnet.solana.com');
if (URL_.includes('mainnet')) throw new Error('refusing a mainnet URL: the drill is for devnet');
const KEYS = opt('--keys', join(root, 'keys/drill'));
const DEPLOYER = opt('--deployer', join(root, 'keys/devnet/deployer-keypair.json'));
// Saved values and the fresh keys (mint and config keypairs) live with the wallets: git-ignored.
const STATE = join(KEYS, 'drill-state.json');
const LOG = join(root, 'scripts/devnet/drill-log.jsonl');
const conn = new web3.Connection(URL_, 'confirmed');
// The SDK (TypeScript, run with --experimental-strip-types) for derived addresses and decoders.
const sdk = await import(pathToFileURL(join(root, 'app/packages/sdk/src/index.ts')).href);
async function itemsMinted() {
  const info = await conn.getAccountInfo(sdk.hookwars.armoryConfigAddress(), 'confirmed');
  if (!info) throw new Error('the armory has no config on this cluster');
  return BigInt(sdk.hookwars.armoryConfigCodec.decode(info.data).itemsMinted);
}

const loadKey = (path) => web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path, 'utf8'))));
const wallet = (name) => {
  const path = join(KEYS, `${name}.json`);
  if (!existsSync(path)) throw new Error(`no drill wallet ${name} (run: wallets ${name})`);
  return loadKey(path);
};
const state = () => (existsSync(STATE) ? JSON.parse(readFileSync(STATE, 'utf8')) : { saved: {}, fresh: {} });
const saveState = (s) => writeFileSync(STATE, JSON.stringify(s, null, 2) + '\n');
const log = (rec) => appendFileSync(LOG, JSON.stringify({ ts: new Date().toISOString(), ...rec }) + '\n');

async function confirm(sig) {
  const bh = await conn.getLatestBlockhash('confirmed');
  const r = await conn.confirmTransaction({ signature: sig, ...bh }, 'confirmed');
  if (r.value.err) throw new Error(`${sig} failed: ${JSON.stringify(r.value.err)}`);
}

async function sendSigned(raw) {
  // Preflight at `processed`: the public RPC's nodes lag one another, and a lookup table created
  // from another node's finalized slot reads as "not a recent slot" on a lagging node's older bank.
  let sig;
  try {
    sig = await conn.sendRawTransaction(raw, { skipPreflight: false, preflightCommitment: 'processed', maxRetries: 5 });
  } catch (e) {
    if (!String(e.message ?? e).includes('is not a recent slot')) throw e;
    sig = await conn.sendRawTransaction(raw, { skipPreflight: true, maxRetries: 5 });
  }
  await confirm(sig);
  return sig;
}

const cmd = argv[0];
if (cmd === 'wallets') {
  mkdirSync(KEYS, { recursive: true, mode: 0o700 });
  for (const name of argv.slice(1).filter((a) => !a.startsWith('--'))) {
    const path = join(KEYS, `${name}.json`);
    if (existsSync(path)) { console.log(`${name} ${loadKey(path).publicKey.toBase58()} (exists)`); continue; }
    const kp = web3.Keypair.generate();
    writeFileSync(path, JSON.stringify(Array.from(kp.secretKey)), { mode: 0o600 });
    console.log(`${name} ${kp.publicKey.toBase58()} (new)`);
  }
} else if (cmd === 'fund') {
  const to = wallet(argv[1]).publicKey;
  const lamports = Math.round(Number(argv[2]) * web3.LAMPORTS_PER_SOL);
  const payer = loadKey(DEPLOYER);
  const tx = new web3.Transaction().add(web3.SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: to, lamports }));
  const sig = await web3.sendAndConfirmTransaction(conn, tx, [payer], { commitment: 'confirmed' });
  log({ step: 'fund', wallet: argv[1], lamports, signature: sig });
  console.log(`funded ${argv[1]} ${lamports} lamports: ${sig}`);
} else if (cmd === 'wrap') {
  // Bridge `wrap_sol` for a drill wallet (the API has no prepare route for it; the site's buy flow
  // expects bridged SOL already held).
  const w = wallet(argv[1]);
  const lamports = BigInt(Math.round(Number(argv[2]) * web3.LAMPORTS_PER_SOL));
  const tx = new web3.Transaction().add(
    sdk.token.createHolding(w.publicKey, sdk.BRIDGED_SOL_MINT, w.publicKey),
    sdk.bridge.wrapSol(w.publicKey, lamports),
  );
  const sig = await web3.sendAndConfirmTransaction(conn, tx, [w], { commitment: 'confirmed' });
  log({ step: 'bridge wrap_sol', wallet: argv[1], lamports: String(lamports), signature: sig });
  console.log(`wrapped ${lamports} for ${argv[1]}: ${sig}`);
} else if (cmd === 'holding') {
  // holding <wallet> <saved mint name>: the wallet's balance of that token, in base units.
  const s6 = state();
  const mint = new web3.PublicKey(s6.saved[argv[2]] ?? argv[2]);
  const info = await conn.getAccountInfo(sdk.hookwars.holdingAddr(mint, wallet(argv[1]).publicKey), 'confirmed');
  console.log(info ? String(sdk.hookwars.holdingCodec.decode(info.data).amount) : '0');
} else if (cmd === 'royalty') {
  // royalty <saved item account name>: the item's royalty holding of bridged SOL, in lamports.
  const item = new web3.PublicKey(state().saved[argv[1]] ?? argv[1]);
  const info = await conn.getAccountInfo(sdk.hookwars.holdingAddr(sdk.BRIDGED_SOL_MINT, sdk.hookwars.royaltyOwner(item)), 'confirmed');
  console.log(info ? String(sdk.hookwars.holdingCodec.decode(info.data).amount) : '0');
} else if (cmd === 'link') {
  // link <operator wallet> <agent wallet> <passport name> <platform> <handle> <post uri>: agents
  // `link_social`, with the ed25519 instruction the agent key signs over the v2 statement (the API
  // has no prepare route for it yet).
  const [, payerName, agentName, passportName, platform, handle, postUri] = argv;
  const payer = wallet(payerName); const agent = wallet(agentName);
  const passport = new web3.PublicKey(state().saved[passportName] ?? passportName);
  const p = sdk.hookwars.passportCodec.decode((await conn.getAccountInfo(passport, 'confirmed')).data);
  const statement = `units agent link v2\npassport: ${passport.toBase58()}\nplatform: ${Number(platform)}\nhandle: ${handle}\nnonce: ${Number(p.linkNonce ?? 0)}`;
  const ed = web3.Ed25519Program.createInstructionWithPrivateKey({ privateKey: agent.secretKey, message: Buffer.from(statement) });
  const link = web3.PublicKey.findProgramAddressSync([Buffer.from('link'), passport.toBuffer(), Buffer.from([Number(platform)])], sdk.hookwars.AGENTS_ID)[0];
  const ix = sdk.hookwars.idlIx('agents', 'link_social', { payer: payer.publicKey, config: sdk.hookwars.agentsConfigAddress(), passport, link, instructions: web3.SYSVAR_INSTRUCTIONS_PUBKEY }, { platform: Number(platform), handle, postUri });
  const sig = await web3.sendAndConfirmTransaction(conn, new web3.Transaction().add(ed, ix), [payer], { commitment: 'confirmed' });
  log({ step: 'agents link_social', passport: passport.toBase58(), platform: Number(platform), handle, signature: sig });
  console.log(`linked: ${sig}`);
} else if (cmd === 'season') {
  // season <number> <starts in secs>: war `propose_season` by the admin (the deployer), TEST weights
  // (raid volume won 1, the m_b e2e's), not owner values (spec 00 section 6). No API route exists.
  const admin = loadKey(DEPLOYER);
  const number = Number(argv[1]);
  const startsAt = BigInt(Math.floor(Date.now() / 1000) + Number(argv[2]));
  const season = sdk.hookwars.seasonAddress(number);
  const weights = { raidVolumeWon: 1n, sieges: 0n, siegeSpend: 0n, timesBesieged: 0n, counterStrikes: 0n, treatySecs: 0n, rivalryWins: 0n };
  const ix = sdk.hookwars.idlIx('war', 'propose_season', { admin: admin.publicKey, config: sdk.hookwars.WAR_CONFIG, season }, { args: { number, startsAt, weights, penalizeBesieged: false } });
  const sig = await web3.sendAndConfirmTransaction(conn, new web3.Transaction().add(ix), [admin], { commitment: 'confirmed' });
  log({ step: 'war propose_season', number, startsAt: String(startsAt), signature: sig });
  console.log(`season ${number} proposed, starts ${startsAt}: ${sig}`);
} else if (cmd === 'loot-table') {
  // loot-table <season>: war `propose_loot_table` by the admin with one TEST entry, the Raid template
  // over its whole schema range (weight 1). open_season needs the season's table (no API route).
  const admin = loadKey(DEPLOYER);
  const season = Number(argv[1]);
  const max = [5000, 300, 1000];
  const ranges = Array.from({ length: 11 }, (_, i) => ({ min: 0, max: max[i] ?? 0 }));
  const ix = sdk.hookwars.idlIx('war', 'propose_loot_table', { admin: admin.publicKey, config: sdk.hookwars.WAR_CONFIG, lootTable: sdk.hookwars.lootTableAddress(season) }, { season, entries: [{ templateId: 1, weight: 1, ranges }] }, [{ pubkey: sdk.hookwars.templateAddress(1), isSigner: false, isWritable: false }]);
  const sig = await web3.sendAndConfirmTransaction(conn, new web3.Transaction().add(ix), [admin], { commitment: 'confirmed' });
  log({ step: 'war propose_loot_table', season, signature: sig });
  console.log(`loot table for season ${season} proposed: ${sig}`);
} else if (cmd === 'sweep') {
  // sweep <wallet>...: unwraps the wallet's bridged SOL and sends its lamports back to the deployer,
  // leaving the fee (the end of a drill).
  const to = loadKey(DEPLOYER).publicKey;
  for (const name of argv.slice(1).filter((a) => !a.startsWith('--'))) {
    const w = wallet(name);
    const h = await conn.getAccountInfo(sdk.hookwars.holdingAddr(sdk.BRIDGED_SOL_MINT, w.publicKey), 'confirmed');
    const held = h ? BigInt(sdk.hookwars.holdingCodec.decode(h.data).amount) : 0n;
    if (held > 0n) {
      const sig = await web3.sendAndConfirmTransaction(conn, new web3.Transaction().add(sdk.bridge.unwrapSol(w.publicKey, held)), [w], { commitment: 'confirmed' });
      log({ step: 'sweep unwrap_sol', wallet: name, amount: String(held), signature: sig });
    }
    const bal = await conn.getBalance(w.publicKey, 'confirmed');
    if (bal <= 5000) { console.log(`${name}: nothing to sweep`); continue; }
    const tx = new web3.Transaction().add(web3.SystemProgram.transfer({ fromPubkey: w.publicKey, toPubkey: to, lamports: bal - 5000 }));
    const sig = await web3.sendAndConfirmTransaction(conn, tx, [w], { commitment: 'confirmed' });
    log({ step: 'sweep', wallet: name, lamports: bal - 5000, signature: sig });
    console.log(`${name}: ${bal - 5000} lamports back to the deployer: ${sig}`);
  }
} else if (cmd === 'save-fresh') {
  // Saves the address of a "$new:" key as <name> (a launch whose --save did not run).
  const s5 = state();
  s5.saved[argv[1]] = web3.Keypair.fromSecretKey(Uint8Array.from(s5.fresh[argv[2]])).publicKey.toBase58(); saveState(s5);
  console.log(`saved ${argv[1]} = ${s5.saved[argv[1]]}`);
} else if (cmd === 'item-account') {
  // Saves "<name>Item", the armory Item account of the item mint saved as <name>.
  const s4 = state();
  s4.saved[`${argv[1]}Item`] = sdk.hookwars.itemAddress(new web3.PublicKey(s4.saved[argv[1]])).toBase58(); saveState(s4);
  console.log(`saved ${argv[1]}Item = ${s4.saved[`${argv[1]}Item`]}`);
} else if (cmd === 'show') {
  const s = state();
  if (existsSync(KEYS)) {
    for (const f of require('node:fs').readdirSync(KEYS).filter((f) => f.endsWith('.json') && f !== 'drill-state.json')) {
      const kp = loadKey(join(KEYS, f));
      console.log(`${f.slice(0, -5)} ${kp.publicKey.toBase58()} ${(await conn.getBalance(kp.publicKey)) / 1e9} SOL`);
    }
  }
  console.log(JSON.stringify(s.saved, null, 2));
} else if (cmd === 'run') {
  const route = argv[1];
  const as = opt('--as', null);
  if (!route || !as) throw new Error('usage: run <route> <json body> --as <wallet>');
  const s = state();
  const owner = wallet(as);
  const fresh = {};
  const subst = (v) => {
    if (typeof v === 'string') {
      if (v.startsWith('$w:')) return wallet(v.slice(3)).publicKey.toBase58();
      if (v.startsWith('$s:')) { const x = s.saved[v.slice(3)]; if (x === undefined) throw new Error(`nothing saved as ${v.slice(3)}`); return x; }
      if (v.startsWith('$new:')) {
        const name = v.slice(5);
        const kp = s.fresh[name] ? web3.Keypair.fromSecretKey(Uint8Array.from(s.fresh[name])) : web3.Keypair.generate();
        s.fresh[name] = Array.from(kp.secretKey); fresh[name] = kp;
        return kp.publicKey.toBase58();
      }
      return v;
    }
    if (Array.isArray(v)) return v.map(subst);
    if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, subst(x)]));
    return v;
  };
  const body = { owner: owner.publicKey.toBase58(), ...subst(JSON.parse(argv[2] ?? '{}')) };
  saveState(s);
  // --item <name>: the call mints one armory item; its mint is the PDA at the counter read now.
  const itemName = opt('--item', null);
  const mintedBefore = itemName ? await itemsMinted() : null;
  const res = await fetch(`${API}/v1/${route}/prepare`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
  const out = await res.json();
  if (!res.ok) { log({ step: route, as, body, error: out }); console.error(`prepare failed ${res.status}: ${JSON.stringify(out)}`); process.exit(1); }
  const extra = (names) => names.map((n) => {
    const kp = n === 'mint' ? (fresh.mint ?? Object.values(fresh)[0]) : (fresh[n] ?? Object.values(fresh)[0]);
    if (!kp) throw new Error(`the API asks for an extra signer "${n}" but the body made no "$new:" key`);
    return kp;
  });
  const sigs = [];
  // --skip <label,...>: stages already landed on an earlier run (a resumed launch), skipped by label.
  // --also <wallet,...>: drill wallets that sign too (an agent key at registration).
  const also = (opt('--also', '') ?? '').split(',').filter(Boolean).map(wallet);
  const skip = new Set((opt('--skip', '') ?? '').split(',').filter(Boolean));
  for (const t of out.transactions ?? []) {
    if (skip.has(t.label)) { console.log(`${t.label}: skipped (landed earlier)`); continue; }
    const bytes = Buffer.from(t.transaction, 'base64');
    let raw;
    if (t.version === 'v0') {
      const tx = web3.VersionedTransaction.deserialize(bytes);
      const bh = await conn.getLatestBlockhash('confirmed');
      tx.message.recentBlockhash = bh.blockhash;
      tx.sign([owner, ...extra(t.extraSigners ?? []), ...also]);
      raw = tx.serialize();
    } else {
      const tx = web3.Transaction.from(bytes);
      tx.recentBlockhash = (await conn.getLatestBlockhash('confirmed')).blockhash;
      tx.sign(owner, ...extra(t.extraSigners ?? []), ...also);
      raw = tx.serialize();
    }
    if (flag('--dry')) { const sim = await conn.simulateTransaction(t.version === 'v0' ? web3.VersionedTransaction.deserialize(raw) : web3.Transaction.from(raw)); console.log(`[dry] ${t.label}: ${JSON.stringify(sim.value.err)} ${sim.value.logs?.slice(-3).join(' | ')}`); continue; }
    try {
      const sig = await sendSigned(raw);
      sigs.push(sig);
      log({ step: route, label: t.label, as, signature: sig });
      console.log(`${t.label}: ${sig}`);
    } catch (e) {
      log({ step: route, label: t.label, as, error: String(e.message ?? e).slice(0, 2000) });
      console.error(`${t.label} failed: ${String(e.message ?? e).slice(0, 2000)}`);
      process.exit(1);
    }
  }
  if (itemName && !flag('--dry')) {
    const itemMint = sdk.hookwars.itemMintAddress(mintedBefore);
    const s3 = state(); s3.saved[itemName] = itemMint.toBase58(); s3.saved[`${itemName}Item`] = sdk.hookwars.itemAddress(itemMint).toBase58(); saveState(s3);
    console.log(`saved ${itemName} = ${s3.saved[itemName]} (item mint ${mintedBefore}), ${itemName}Item = ${s3.saved[`${itemName}Item`]}`);
  }
  const save = opt('--save', null);
  if (save) {
    for (const spec of save.split(',')) {
      const [name, path] = spec.split('=');
      let v;
      if (path) v = path.split('.').reduce((o, k) => (o == null ? o : o[k]), out);
      else v = (fresh.mint ?? Object.values(fresh)[0])?.publicKey.toBase58();
      const s2 = state(); s2.saved[name] = v; saveState(s2);
      console.log(`saved ${name} = ${v}`);
    }
  }
  if (out.quote) console.log(`quote ${JSON.stringify(out.quote)}`);
} else {
  console.log('commands: wallets, fund, run, show (see the header)');
}
