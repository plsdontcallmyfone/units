// Provenance of the model that ran: every action's memo carries the provider, the model id and the
// sha256 of the exact prompt and the exact output, so anyone holding the published prompt log can
// check which model proposed the action.
import { createHash } from 'node:crypto';
import { obj, type Obj } from './memo.ts';
import type { ModelReply, ModelRequest } from './models/types.ts';

export interface Provenance {
  provider: string;
  model: string;
  /** Hex sha256 of the canonical prompt (`promptBytes`). */
  promptHash: string;
  /** Hex sha256 of the output text (UTF-8). */
  outputHash: string;
}

const sha256hex = (b: Buffer | string): string => createHash('sha256').update(b).digest('hex');

/** The canonical prompt: fixed key order, no whitespace, the provider and model included. */
export function promptBytes(provider: string, model: string, req: ModelRequest): Buffer {
  const canon = {
    provider, model,
    system: req.system,
    messages: req.messages.map((m) => ({ role: m.role, content: m.content })),
    max_tokens: req.maxTokens,
    temperature: req.temperature,
  };
  return Buffer.from(JSON.stringify(canon), 'utf8');
}

export function provenanceOf(req: ModelRequest, reply: ModelReply): Provenance {
  return { provider: reply.provider, model: reply.model, promptHash: sha256hex(promptBytes(reply.provider, reply.model, req)), outputHash: sha256hex(Buffer.from(reply.text, 'utf8')) };
}

/**
 * Hex characters of each hash carried in a memo: the first 16 bytes of the sha256. A memo has
 * `memo_max_bytes` of room and a reply in a thread already spends most of it on message ids; the
 * full hashes stay in the published prompt log (`prompts.jsonl`).
 */
export const MEMO_HASH_HEX = 32;

/** The memo body field `pv`. */
export function provenanceValue(p: Provenance): Obj {
  return obj([['p', p.provider], ['m', p.model], ['ph', p.promptHash.slice(0, MEMO_HASH_HEX)], ['oh', p.outputHash.slice(0, MEMO_HASH_HEX)]]);
}

/** Checks a memo's `pv` against a full provenance record. */
export function memoMatches(pv: { p: string; m: string; ph: string; oh: string }, p: Provenance): boolean {
  return pv.p === p.provider && pv.m === p.model && pv.ph === p.promptHash.slice(0, MEMO_HASH_HEX) && pv.oh === p.outputHash.slice(0, MEMO_HASH_HEX);
}

/** Checks a recorded provenance against a published prompt and output. */
export function verifyProvenance(p: Provenance, req: ModelRequest, outputText: string): boolean {
  return p.promptHash === sha256hex(promptBytes(p.provider, p.model, req)) && p.outputHash === sha256hex(Buffer.from(outputText, 'utf8'));
}

/** Provenance of a deterministic step (no model ran), e.g. a crank or a facts-only status. */
export function ruleProvenance(rule: string, input: string): Provenance {
  return { provider: 'rule', model: rule, promptHash: sha256hex(input), outputHash: sha256hex('') };
}
