/**
 * Starts the Hook Lab service. Every setting is the operator's (none is a protocol parameter):
 *
 *   HOOKLAB_PORT (9980), HOOKLAB_HOST (127.0.0.1), HOOKLAB_DATA (./hooklab-data),
 *   HOOKLAB_BIN (hooklab on PATH), HOOKLAB_KEY (the lab's keypair file; required),
 *   HOOKLAB_SANDBOX (command prefix for every run, split on spaces; required unless
 *   HOOKLAB_UNSANDBOXED=1), HOOKLAB_TIMEOUT_MS, HOOKLAB_WORKERS, HOOKLAB_CHECK_ARGS,
 *   HOOKLAB_MAX_GZ, HOOKLAB_MAX_UNPACKED, HOOKLAB_MAX_FILES, HOOKLAB_MAX_QUEUED,
 *   HOOKLAB_RATE_SUBMIT_CAPACITY, HOOKLAB_RATE_SUBMIT_PER_SEC, HOOKLAB_TRUST_PROXY,
 *   HOOKLAB_FETCH_SANDBOX (prefix of the git fetch, which needs the network; default HOOKLAB_SANDBOX),
 *   HOOKLAB_CARGO_HOME (a cargo home primed with sandbox/prime-cache.sh; the build then runs
 *   offline), HOOKLAB_BUILD_PATH (PATH of the build stage). See sandbox/ for the scripts.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { Lab } from './lab.ts';
import { createLabServer, RateLimiter } from './server.ts';
import { publicKeyOf } from './key.ts';

const env = (k: string, d: string): string => process.env[k] ?? d;
const num = (k: string, d: number): number => { const v = Number(process.env[k]); return Number.isFinite(v) && v > 0 ? v : d; };

const keyPath = process.env.HOOKLAB_KEY;
if (!keyPath) {
  console.error('hooklab: set HOOKLAB_KEY to the lab signing keypair file');
  process.exit(2);
}
const signer = publicKeyOf(JSON.parse(readFileSync(keyPath, 'utf8')) as number[]);
const unsandboxed = process.env.HOOKLAB_UNSANDBOXED === '1';
const sandbox = env('HOOKLAB_SANDBOX', '').split(' ').filter(Boolean);
if (!unsandboxed && sandbox.length === 0) {
  console.warn('hooklab: no HOOKLAB_SANDBOX: submissions are accepted but every run ends in an error until one is set');
}
const lab = new Lab({
  dataDir: env('HOOKLAB_DATA', './hooklab-data'),
  bin: env('HOOKLAB_BIN', 'hooklab'),
  keyPath,
  sandbox,
  unsandboxed,
  fetchSandbox: env('HOOKLAB_FETCH_SANDBOX', '').split(' ').filter(Boolean),
  cargoHome: process.env.HOOKLAB_CARGO_HOME || null,
  buildPath: process.env.HOOKLAB_BUILD_PATH || '',
  timeoutMs: num('HOOKLAB_TIMEOUT_MS', 20 * 60 * 1000),
  checkArgs: env('HOOKLAB_CHECK_ARGS', '').split(' ').filter(Boolean),
  workers: num('HOOKLAB_WORKERS', 1),
  tar: {
    maxCompressed: num('HOOKLAB_MAX_GZ', 2 * 1024 * 1024),
    maxUnpacked: num('HOOKLAB_MAX_UNPACKED', 16 * 1024 * 1024),
    maxFiles: num('HOOKLAB_MAX_FILES', 500),
    maxPath: 200,
  },
});
lab.recover(readdirSync(lab.cfg.dataDir));
const server = createLabServer({
  lab,
  signer,
  submitLimit: new RateLimiter(num('HOOKLAB_RATE_SUBMIT_CAPACITY', 5), num('HOOKLAB_RATE_SUBMIT_PER_SEC', 1 / 600)),
  readLimit: new RateLimiter(120, 4),
  trustProxy: process.env.HOOKLAB_TRUST_PROXY === '1',
  maxQueued: num('HOOKLAB_MAX_QUEUED', 50),
});
const port = num('HOOKLAB_PORT', 9980);
server.listen(port, env('HOOKLAB_HOST', '127.0.0.1'), () => {
  console.log(`hooklab service on :${port}, signer ${signer}`);
});
