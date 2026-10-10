// Security review 3 (M-10, L-9): the runtime refuses prepared transactions that reach past their
// route, plain-http APIs, private rules hosts, and keeps the RPC URL out of logs.
import { describe, expect, it } from 'vitest';
import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { hookwars, token } from '@hookwars/sdk';
import { readEnv } from './env.ts';
import { checkApiUrl, checkRulesUri, preparedProblems, privateAddress } from './guard.ts';
import { memoryLogger } from './log.ts';
import { tick } from './loop.ts';
import { replay } from './models/stub.ts';
import { ApiRouter } from './router.ts';
import { AGENT, kit, SentSender } from './testkit.ts';

const TOKEN_MINT = new PublicKey(new Uint8Array(32).fill(12));
const OTHER = new PublicKey(new Uint8Array(32).fill(13));
const VAULT = new PublicKey(new Uint8Array(32).fill(14));
const approve = (owner: PublicKey) => token.approve(owner, hookwars.holdingAddr(TOKEN_MINT, owner), OTHER, 2n ** 64n - 1n);

describe('prepared transactions (M-10)', () => {
  it('accepts what the routes build', () => {
    const swap = new TransactionInstruction({ programId: hookwars.SWAP_ID, keys: [{ pubkey: VAULT, isSigner: true, isWritable: true }], data: Buffer.from([9]) });
    expect(preparedProblems('buy/prepare', [token.createHolding(VAULT, TOKEN_MINT, VAULT), swap], AGENT.publicKey, VAULT)).toEqual([]);
    const settle = new TransactionInstruction({ programId: hookwars.ITEMS_ID, keys: [{ pubkey: AGENT.publicKey, isSigner: true, isWritable: true }], data: Buffer.from([1]) });
    expect(preparedProblems('settle/prepare', [settle], AGENT.publicKey, VAULT)).toEqual([]);
  });

  it('refuses a system transfer, a token approve, an agent-signed swap and an unknown route', () => {
    const drain = SystemProgram.transfer({ fromPubkey: AGENT.publicKey, toPubkey: OTHER, lamports: 1 });
    expect(preparedProblems('settle/prepare', [drain], AGENT.publicKey, VAULT)[0]).toMatch(/never needs/);
    expect(preparedProblems('buy/prepare', [approve(VAULT)], AGENT.publicKey, VAULT)[0]).toMatch(/other than create_holding/);
    expect(preparedProblems('settle/prepare', [token.transfer(AGENT.publicKey, hookwars.holdingAddr(TOKEN_MINT, AGENT.publicKey), hookwars.holdingAddr(TOKEN_MINT, OTHER), TOKEN_MINT, 5n)], AGENT.publicKey, VAULT)[0]).toMatch(/other than create_holding/);
    const agentSwap = new TransactionInstruction({ programId: hookwars.SWAP_ID, keys: [{ pubkey: AGENT.publicKey, isSigner: true, isWritable: true }], data: Buffer.from([9]) });
    expect(preparedProblems('buy/prepare', [agentSwap], AGENT.publicKey, VAULT)[0]).toMatch(/could spend its funds/);
    expect(preparedProblems('market/buy/prepare', [], AGENT.publicKey, VAULT)[0]).toMatch(/no program allowlist/);
  });

  it('the loop refuses a crank whose prepared transaction transfers the agent key\'s SOL', async () => {
    const k = kit(replay([{ actions: [{ type: 'crank', route: 'settle/prepare', body: {} }] }]));
    k.router.builders.set('settle/prepare', () => [SystemProgram.transfer({ fromPubkey: AGENT.publicKey, toPubkey: OTHER, lamports: 1_000_000_000 })]);
    const r = await tick(k.rt);
    expect(r.results[0]!.ok).toBe(false);
    expect(r.results[0]!.refusals![0]).toMatch(/never needs/);
    expect(k.sender.submitted).toHaveLength(0);
  });

  it('the loop refuses a buy whose prepared transaction approves a delegate on the vault', async () => {
    const k = kit(replay([{ actions: [{ type: 'trade', side: 'buy', mint: TOKEN_MINT.toBase58(), amountIn: '100000000', minOut: '995000', reason: 'a test of the guard' }] }]), {}, '{"v":1}');
    k.chain.mints.set(TOKEN_MINT.toBase58(), { creator: OTHER.toBase58(), itemAuthors: [] });
    k.router.quotes.set(TOKEN_MINT.toBase58(), 1_000_000n);
    k.router.builders.set('buy/prepare', (b) => [approve(new PublicKey(b.owner as string))]);
    k.rt.sender = new SentSender();
    const r = await tick(k.rt);
    expect(r.results[0]!.ok).toBe(false);
    expect(r.results[0]!.refusals!.join(' ')).toMatch(/other than create_holding/);
  });
});

describe('URLs (M-10, L-9)', () => {
  it('the API must be https, or http on this machine', () => {
    expect(() => checkApiUrl('http://api.example.org')).toThrow(/https/);
    expect(checkApiUrl('https://api.example.org')).toBe('https://api.example.org');
    expect(checkApiUrl('http://127.0.0.1:9961')).toBe('http://127.0.0.1:9961');
    expect(() => new ApiRouter('http://api.example.org', null as never)).toThrow(/https/);
  });

  it('rules documents come over https from public addresses only', async () => {
    await expect(checkRulesUri('http://rules.example.org/r.json', async () => ['93.184.216.34'])).rejects.toThrow(/https/);
    await expect(checkRulesUri('https://169.254.169.254/latest/meta-data')).rejects.toThrow(/private/);
    await expect(checkRulesUri('https://rules.example.org/r.json', async () => ['10.0.0.7'])).rejects.toThrow(/private/);
    await expect(checkRulesUri('https://rules.example.org/r.json', async () => ['93.184.216.34', '::1'])).rejects.toThrow(/private/);
    expect((await checkRulesUri('https://rules.example.org/r.json', async () => ['93.184.216.34'])).hostname).toBe('rules.example.org');
    for (const ip of ['127.0.0.1', '10.1.2.3', '172.20.0.1', '192.168.1.1', '100.64.0.1', '0.0.0.0', 'fd00::1', 'fe80::1', '::ffff:10.0.0.1']) expect(privateAddress(ip)).toBe(true);
    for (const ip of ['93.184.216.34', '2606:4700::1111']) expect(privateAddress(ip)).toBe(false);
  });

  it('the RPC URL and its key never reach a log line', () => {
    const url = 'https://rpc.example.org/v2/abcdefghijklmnopqrstuvwx?api-key=sekret-value-123';
    const env = readEnv({ UNITS_RPC_URL: url });
    const log = memoryLogger(env.secrets());
    log.info('rpc', { url, note: 'key sekret-value-123 and path abcdefghijklmnopqrstuvwx' });
    expect(log.lines.join('\n')).not.toMatch(/sekret-value-123|abcdefghijklmnopqrstuvwx/);
  });
});
