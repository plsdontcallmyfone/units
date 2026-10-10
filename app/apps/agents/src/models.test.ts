import { inspect } from 'node:util';
import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { readEnv, Secret } from './env.ts';
import { makeLogger, memoryLogger, redact } from './log.ts';
import { ANTHROPIC_VERSION, DEFAULT_CLAUDE_MODEL } from './models/anthropic.ts';
import { ModelConfigError, requiredEnv, resolveModel } from './models/registry.ts';
import { replay, stubModel } from './models/stub.ts';
import type { Fetch, ModelRequest } from './models/types.ts';
import { memoMatches, promptBytes, provenanceOf, provenanceValue, ruleProvenance, verifyProvenance } from './provenance.ts';
import { signStatement, statementDigest, verifyStatement } from './statements.ts';
import { encodeValue } from './memo.ts';

const KEY = 'sk-test-0123456789abcdef0123456789';
const req: ModelRequest = { system: 'sys', messages: [{ role: 'user', content: 'hi' }], maxTokens: 64, temperature: 0 };

function fakeFetch(status: number, body: unknown) {
  const calls: { url: string; headers: Record<string, string>; body: string }[] = [];
  const f: Fetch = async (url, init) => { calls.push({ url, headers: init.headers, body: init.body }); return { ok: status < 300, status, text: async () => (typeof body === 'string' ? body : JSON.stringify(body)) }; };
  return { f, calls };
}

describe('model registry', () => {
  it('calls the Anthropic Messages API with the key only in its header', async () => {
    const env = readEnv({ ANTHROPIC_API_KEY: KEY });
    const { f, calls } = fakeFetch(200, { model: 'claude-x', content: [{ type: 'text', text: '{"actions":[]}' }] });
    const m = resolveModel({ provider: 'anthropic' }, env, { fetch: f });
    expect(m.model).toBe(DEFAULT_CLAUDE_MODEL);
    const r = await m.complete(req);
    expect(r).toEqual({ provider: 'anthropic', model: 'claude-x', text: '{"actions":[]}' });
    expect(calls[0]!.url).toBe('https://api.anthropic.com/v1/messages');
    expect(calls[0]!.headers['x-api-key']).toBe(KEY);
    expect(calls[0]!.headers['anthropic-version']).toBe(ANTHROPIC_VERSION);
    expect(JSON.parse(calls[0]!.body)).toEqual({ model: DEFAULT_CLAUDE_MODEL, max_tokens: 64, temperature: 0, system: 'sys', messages: [{ role: 'user', content: 'hi' }] });
    expect(calls[0]!.body).not.toContain(KEY);
  });

  it('takes the Claude model id from the config, then UNITS_ANTHROPIC_MODEL, then the default', () => {
    const env = readEnv({ UNITS_ANTHROPIC_API_KEY: KEY, UNITS_ANTHROPIC_MODEL: 'claude-sonnet-5-5' });
    expect(resolveModel({ provider: 'anthropic' }, env).model).toBe('claude-sonnet-5-5');
    expect(resolveModel({ provider: 'anthropic', model: 'claude-haiku-5-5' }, env).model).toBe('claude-haiku-5-5');
  });

  it('runs any OpenAI-compatible provider from a base URL, key and model id', async () => {
    const env = readEnv({ UNITS_DEEPSEEK_API_KEY: KEY, UNITS_DEEPSEEK_BASE_URL: 'https://llm.example/v1/', UNITS_DEEPSEEK_MODEL: 'deep-1' });
    const { f, calls } = fakeFetch(200, { model: 'deep-1', choices: [{ message: { content: 'ok' } }] });
    const r = await resolveModel({ provider: 'deepseek' }, env, { fetch: f }).complete(req);
    expect(r).toEqual({ provider: 'deepseek', model: 'deep-1', text: 'ok' });
    expect(calls[0]!.url).toBe('https://llm.example/v1/chat/completions');
    expect(calls[0]!.headers.authorization).toBe(`Bearer ${KEY}`);
    expect(JSON.parse(calls[0]!.body).messages).toEqual([{ role: 'system', content: 'sys' }, { role: 'user', content: 'hi' }]);
  });

  it('names the missing variable and refuses plain http to a remote host', () => {
    expect(() => resolveModel({ provider: 'anthropic' }, readEnv({}))).toThrow(/UNITS_ANTHROPIC_API_KEY/);
    expect(() => resolveModel({ provider: 'qwen' }, readEnv({ UNITS_QWEN_API_KEY: KEY }))).toThrow(/UNITS_QWEN_BASE_URL/);
    expect(() => resolveModel({ provider: 'kimi' }, readEnv({ UNITS_KIMI_API_KEY: KEY, UNITS_KIMI_BASE_URL: 'https://x/v1' }))).toThrow(/UNITS_KIMI_MODEL/);
    expect(() => resolveModel({ provider: 'glm', model: 'g' }, readEnv({ UNITS_GLM_API_KEY: KEY, UNITS_GLM_BASE_URL: 'http://remote.example/v1' }))).toThrow(ModelConfigError);
    expect(resolveModel({ provider: 'llama', model: 'l' }, readEnv({ UNITS_LLAMA_API_KEY: KEY, UNITS_LLAMA_BASE_URL: 'http://127.0.0.1:8000/v1' })).provider).toBe('llama');
    expect(requiredEnv({ provider: 'minimax' })).toEqual(['UNITS_MINIMAX_API_KEY', 'UNITS_MINIMAX_BASE_URL', 'UNITS_MINIMAX_MODEL']);
    expect(requiredEnv({ provider: 'openai', model: 'o' })).toEqual(['UNITS_OPENAI_API_KEY']);
    expect(requiredEnv({ provider: 'stub' })).toEqual([]);
  });

  it('keeps the key out of provider errors', async () => {
    const env = readEnv({ ANTHROPIC_API_KEY: KEY });
    const { f } = fakeFetch(401, { error: { message: 'invalid x-api-key' } });
    await expect(resolveModel({ provider: 'anthropic' }, env, { fetch: f }).complete(req)).rejects.toThrow(/anthropic: .*invalid x-api-key/);
    const err = await resolveModel({ provider: 'anthropic' }, env, { fetch: f }).complete(req).catch((e: Error) => e);
    expect(String((err as Error).message)).not.toContain(KEY);
  });

  it('has a deterministic stub that never touches the network', async () => {
    const s = stubModel(replay([{ actions: [] }, 'raw']));
    expect((await s.complete(req)).text).toBe('{"actions":[]}');
    expect((await s.complete(req)).text).toBe('raw');
    expect((await s.complete(req)).text).toBe('{"actions":[]}');
    expect(s.calls).toHaveLength(3);
  });
});

describe('secrets never reach logs', () => {
  it('redacts secret values, secret-looking keys and 64-byte key arrays', () => {
    const env = readEnv({ UNITS_OPENAI_API_KEY: KEY });
    const log = memoryLogger(env.secrets());
    const kp = Keypair.generate();
    log.log('info', 'x', { note: `header Bearer ${KEY}`, apiKey: 'whatever', secretKey: Array.from(kp.secretKey), raw: kp.secretKey, nested: { privateKey: 'p' }, key: new Secret(KEY) });
    const line = log.lines[0]!;
    expect(line).not.toContain(KEY);
    expect(line).not.toContain(Buffer.from(kp.secretKey).toString('hex'));
    expect(JSON.parse(line)).toMatchObject({ note: 'header Bearer [redacted]', apiKey: '[redacted]', secretKey: '[redacted]', raw: '[redacted]', nested: { privateKey: '[redacted]' }, key: '[redacted]' });
  });

  it('wraps keys so printing them shows nothing', () => {
    const s = new Secret(KEY);
    expect(`${s}`).toBe('[redacted]');
    expect(JSON.stringify({ s })).toBe('{"s":"[redacted]"}');
    expect(inspect(s)).toBe('[redacted]');
    expect(redact({ s }, [])).toEqual({ s: '[redacted]' });
    const env = readEnv({ ANTHROPIC_API_KEY: KEY });
    expect(JSON.stringify(env.provider('anthropic'))).not.toContain(KEY);
    const lines: string[] = [];
    makeLogger({ write: (l) => lines.push(l), secrets: env.secrets }).child({ agent: 'a' }).log('error', 'e', { msg: KEY });
    expect(lines[0]).not.toContain(KEY);
  });
});

describe('provenance', () => {
  it('hashes the exact prompt and output, and any change shows', () => {
    const p = provenanceOf(req, { provider: 'anthropic', model: 'claude-x', text: 'out' });
    expect(verifyProvenance(p, req, 'out')).toBe(true);
    expect(verifyProvenance(p, req, 'out ')).toBe(false);
    expect(verifyProvenance(p, { ...req, temperature: 1 }, 'out')).toBe(false);
    expect(verifyProvenance({ ...p, model: 'claude-y' }, req, 'out')).toBe(false);
    expect(promptBytes('a', 'b', req).toString()).toBe('{"provider":"a","model":"b","system":"sys","messages":[{"role":"user","content":"hi"}],"max_tokens":64,"temperature":0}');
    expect(encodeValue(provenanceValue(p))).toBe(`{"p":"anthropic","m":"claude-x","ph":"${p.promptHash.slice(0, 32)}","oh":"${p.outputHash.slice(0, 32)}"}`);
    expect(memoMatches({ p: 'anthropic', m: 'claude-x', ph: p.promptHash.slice(0, 32), oh: p.outputHash.slice(0, 32) }, p)).toBe(true);
    expect(memoMatches({ p: 'anthropic', m: 'claude-x', ph: p.outputHash.slice(0, 32), oh: p.outputHash.slice(0, 32) }, p)).toBe(false);
    expect(ruleProvenance('crank', 'x').provider).toBe('rule');
  });
});

describe('domain-separated statements', () => {
  it('verifies only for the same signer, purpose and payload', () => {
    const kp = Keypair.generate();
    const payload = Buffer.from('passport P links handle @h');
    const sig = signStatement(kp, 'link-proof', payload);
    expect(verifyStatement(kp.publicKey, 'link-proof', payload, sig)).toBe(true);
    expect(verifyStatement(kp.publicKey, 'offer', payload, sig)).toBe(false);
    expect(verifyStatement(kp.publicKey, 'link-proof', Buffer.from('other'), sig)).toBe(false);
    expect(verifyStatement(Keypair.generate().publicKey, 'link-proof', payload, sig)).toBe(false);
    expect(statementDigest('offer', payload).equals(statementDigest('link-proof', payload))).toBe(false);
    expect(() => statementDigest('anything' as 'offer', payload)).toThrow();
  });
});
