#!/usr/bin/env node
// units-agents: seed and register agents, post directives, run agents once or in a loop.
// Everything is a dry run (built and simulated, not sent) unless --send is given.
import { createHash } from 'node:crypto';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { Connection, Keypair, PublicKey, type AddressLookupTableAccount } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { RpcChain } from './chain.ts';
import { loadConfig, loadKeypair, type AgentConfig } from './config.ts';
import { setDirectiveInstructions, type DirectiveConstraints } from './directive.ts';
import { readEnv, type RuntimeEnv } from './env.ts';
import { pickPort, startHealth, type AgentHealth } from './health.ts';
import { makeLogger, type Logger } from './log.ts';
import { tick, type Runtime } from './loop.ts';
import { requiredEnv, resolveModel } from './models/registry.ts';
import { ApiRouter } from './router.ts';
import { RpcSender } from './sender.ts';
import { fileStore } from './state.ts';

interface Args { cmd: string; pos: string[]; flags: Map<string, string | true> }

export function parseArgs(argv: string[]): Args {
  const [cmd = 'help', ...rest] = argv;
  const pos: string[] = [];
  const flags = new Map<string, string | true>();
  for (let i = 0; i < rest.length; i++) {
    const a = rest[i]!;
    if (a.startsWith('--')) {
      const [k, v] = a.slice(2).split('=', 2) as [string, string | undefined];
      if (v !== undefined) flags.set(k, v);
      else if (rest[i + 1] !== undefined && !rest[i + 1]!.startsWith('--') && !['send', 'once', 'loop', 'mock'].includes(k)) flags.set(k, rest[++i]!);
      else flags.set(k, true);
    } else pos.push(a);
  }
  return { cmd, pos, flags };
}

const HELP = `units-agents <command>

  seed <dir> <name>                     make an agent keypair at <dir>/<name>.json (mode 600) and print its address
  env <config.json>...                  list the environment variables these agents need (names only)
  register <config.json> --operator-keypair <path> --limits <limits.json> [--name-on-chain <text>] [--send]
                                        register_passport (operator and agent key sign) and init_policy
  directive <config.json> --operator-keypair <path> --seq <n> --rules <file> --rules-uri <uri> --constraints <json file> [--send]
                                        post a directive: the memo signed by the operator and set_directive in one transaction
  run <config.json>... [--once | --loop] [--send]
                                        run agents; dry run unless --send; health on a free port in 9970 to 9979

Environment: UNITS_RPC_URL, UNITS_API_URL, UNITS_PROTOCOL_LOOKUP_TABLE, UNITS_HEALTH_PORT, and per model
provider UNITS_<ID>_API_KEY, UNITS_<ID>_BASE_URL, UNITS_<ID>_MODEL (README.md).`;

function need(env: RuntimeEnv, k: 'rpcUrl' | 'apiUrl', name: string): string {
  const v = env[k];
  if (!v) throw new Error(`${name} is not set`);
  return v;
}

async function tables(conn: Connection, env: RuntimeEnv): Promise<AddressLookupTableAccount[]> {
  if (!env.lookupTable) return [];
  const t = (await conn.getAddressLookupTable(new PublicKey(env.lookupTable), { commitment: 'confirmed' })).value;
  return t ? [t] : [];
}

function report(log: Logger, what: string, r: { sent: boolean; signature: string | null; error: string | null; unitsConsumed: number | null; logs: string[] }) {
  log.log(r.error ? 'error' : 'info', what, { sent: r.sent, signature: r.signature, error: r.error, units: r.unitsConsumed, ...(r.error ? { logs: r.logs.slice(-12) } : {}) });
  if (r.error) process.exitCode = 1;
}

async function cmdRegister(a: Args, env: RuntimeEnv, log: Logger) {
  const cfg = loadConfig(a.pos[0] ?? '');
  const operator = loadKeypair(String(a.flags.get('operator-keypair')));
  if (!operator.publicKey.equals(cfg.operator)) throw new Error('the operator keypair is not the config operator');
  const agent = loadKeypair(cfg.agentKeypairPath);
  const lim = JSON.parse(readFileSync(String(a.flags.get('limits')), 'utf8')) as { perActionLamports: string; perDayLamports: string; tracked?: { mint: string; perAction: string; perDay: string }[]; targets: string[] };
  const conn = new Connection(need(env, 'rpcUrl', 'UNITS_RPC_URL'), 'confirmed');
  const chain = new RpcChain(conn);
  const ac = await chain.agentsConfig();
  if (!ac) throw new Error('no agents config on this cluster');
  const passport = hookwars.passportAddress(cfg.operator, cfg.passportIndex);
  const nameOnChain = typeof a.flags.get('name-on-chain') === 'string' ? String(a.flags.get('name-on-chain')) : cfg.name;
  const ixs = [
    hookwars.agentsRegisterPassport(cfg.operator, agent.publicKey, cfg.operator, ac.feeCollector, cfg.passportIndex, { name: nameOnChain, avatarUri: '', bioUri: '', hireUri: '', kinds: 0, creditAgentId: null }),
    hookwars.agentsInitPolicy(cfg.operator, cfg.operator, passport, {
      perActionLamports: BigInt(lim.perActionLamports), perDayLamports: BigInt(lim.perDayLamports),
      tracked: (lim.tracked ?? []).map((t) => ({ mint: new PublicKey(t.mint), perAction: BigInt(t.perAction), perDay: BigInt(t.perDay) })),
      targets: lim.targets.map((t) => new PublicKey(t)),
    }),
  ];
  const sender = new RpcSender(conn, operator, a.flags.get('send') === true, await tables(conn, env), [agent]);
  log.log('info', 'register', { passport: passport.toBase58(), agentKey: agent.publicKey.toBase58(), vault: hookwars.agentVaultAddress(passport).toBase58() });
  report(log, 'register-result', await sender.submit(ixs, []));
}

async function cmdDirective(a: Args, env: RuntimeEnv, log: Logger) {
  const cfg = loadConfig(a.pos[0] ?? '');
  const operator = loadKeypair(String(a.flags.get('operator-keypair')));
  if (!operator.publicKey.equals(cfg.operator)) throw new Error('the operator keypair is not the config operator');
  const seq = Number(a.flags.get('seq'));
  if (!Number.isSafeInteger(seq) || seq < 0) throw new Error('--seq: the next directive sequence');
  const rules = readFileSync(String(a.flags.get('rules')));
  const h = createHash('sha256').update(rules).digest('hex');
  const uri = String(a.flags.get('rules-uri'));
  const k = JSON.parse(readFileSync(String(a.flags.get('constraints')), 'utf8')) as { maxSpendPerAction: string; maxSpendPerDay: string; allowedTargets: string[]; allowedAccessModes: number; maxLicencePrice: string; frozen: boolean };
  const constraints: DirectiveConstraints = { maxSpendPerAction: BigInt(k.maxSpendPerAction), maxSpendPerDay: BigInt(k.maxSpendPerDay), allowedTargets: k.allowedTargets.map((t) => new PublicKey(t)), allowedAccessModes: k.allowedAccessModes, maxLicencePrice: BigInt(k.maxLicencePrice), frozen: k.frozen === true };
  const conn = new Connection(need(env, 'rpcUrl', 'UNITS_RPC_URL'), 'confirmed');
  const passport = hookwars.passportAddress(cfg.operator, cfg.passportIndex);
  log.log('info', 'directive', { passport: passport.toBase58(), seq, rulesUri: uri, rulesSha256: h, note: 'publish the rules file at rules-uri byte for byte; the agent checks its sha256' });
  const sender = new RpcSender(conn, operator, a.flags.get('send') === true, await tables(conn, env));
  report(log, 'directive-result', await sender.submit(setDirectiveInstructions(cfg.operator, passport, seq, constraints, uri, h), []));
}

async function cmdRun(a: Args, env: RuntimeEnv, log: Logger) {
  const send = a.flags.get('send') === true;
  const loop = a.flags.get('loop') === true;
  const conn = new Connection(need(env, 'rpcUrl', 'UNITS_RPC_URL'), 'confirmed');
  const extra = await tables(conn, env);
  const chain = new RpcChain(conn);
  const router = new ApiRouter(need(env, 'apiUrl', 'UNITS_API_URL'), conn);
  const configs: AgentConfig[] = a.pos.map(loadConfig);
  if (!configs.length) throw new Error('run: give at least one agent config');
  const health = new Map<string, AgentHealth>();
  const runtimes: Runtime[] = configs.map((cfg) => {
    const kp = loadKeypair(cfg.agentKeypairPath);
    const model = resolveModel(cfg.model, env);
    const alog = log.child({ agent: cfg.name });
    mkdirSync(cfg.stateDir, { recursive: true });
    health.set(cfg.name, { name: cfg.name, passport: null, lastTickAt: null, directiveSeq: null, halted: null, lastError: null, ticks: 0, sends: send, provider: model.provider, model: model.model });
    return {
      cfg, agentKey: kp.publicKey, model, chain, router, sender: new RpcSender(conn, kp, send, extra), store: fileStore(cfg.stateDir), log: alog,
      recordPrompt: (rec) => appendFileSync(join(cfg.stateDir, 'prompts.jsonl'), JSON.stringify(rec) + '\n'),
    };
  });
  const port = await pickPort(env.healthPort);
  const server = await startHealth(port, () => [...health.values()]);
  log.log('info', 'started', { agents: configs.map((c) => c.name), send, loop, healthPort: port });
  const once = async (rt: Runtime) => {
    const h = health.get(rt.cfg.name)!;
    try {
      const r = await tick(rt);
      Object.assign(h, { passport: r.passport, lastTickAt: r.now, directiveSeq: r.directiveSeq, halted: r.halted, lastError: null, ticks: h.ticks + 1 });
      rt.log.log('info', 'tick', { directive: r.directiveSeq, halted: r.halted, results: r.results, dropped: r.dropped, provenance: r.provenance });
    } catch (e) {
      h.lastError = (e as Error).message;
      rt.log.log('error', 'tick-failed', { error: (e as Error).message });
    }
  };
  if (!loop) { for (const rt of runtimes) await once(rt); server.close(); return; }
  let stop = false;
  const onSignal = () => { stop = true; log.log('info', 'stopping'); };
  process.once('SIGINT', onSignal); process.once('SIGTERM', onSignal);
  const next = new Map(runtimes.map((rt) => [rt.cfg.name, 0]));
  while (!stop) {
    const t = Date.now();
    for (const rt of runtimes) if (t >= next.get(rt.cfg.name)!) { await once(rt); next.set(rt.cfg.name, Date.now() + rt.cfg.tickSecs * 1000); }
    await new Promise((ok) => setTimeout(ok, 1000));
  }
  server.close();
}

export async function main(argv: string[]): Promise<void> {
  const a = parseArgs(argv);
  const env = readEnv();
  const log = makeLogger({ secrets: env.secrets });
  switch (a.cmd) {
    case 'seed': {
      const [dir, name] = a.pos;
      if (!dir || !name || !/^[a-z0-9-]{1,32}$/.test(name)) throw new Error('seed <dir> <name>');
      const path = join(dir, `${name}.json`);
      if (existsSync(path)) throw new Error(`${path} exists; not overwriting a key`);
      mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
      const kp = Keypair.generate();
      writeFileSync(path, JSON.stringify(Array.from(kp.secretKey)), { mode: 0o600 });
      log.log('info', 'seeded', { path, agentKey: kp.publicKey.toBase58() });
      return;
    }
    case 'env': {
      for (const p of a.pos) { const c = loadConfig(p); log.log('info', 'env', { agent: c.name, provider: c.model.provider, needs: ['UNITS_RPC_URL', 'UNITS_API_URL', ...requiredEnv(c.model)], optional: ['UNITS_PROTOCOL_LOOKUP_TABLE', 'UNITS_HEALTH_PORT'] }); }
      return;
    }
    case 'register': return cmdRegister(a, env, log);
    case 'directive': return cmdDirective(a, env, log);
    case 'run': return cmdRun(a, env, log);
    default: process.stdout.write(HELP + '\n');
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main(process.argv.slice(2)).catch((e: Error) => { process.stderr.write(`units-agents: ${e.message}\n`); process.exit(1); });
}
