// A deterministic model for tests and dry runs: no network, the same reply for the same request.
import { createHash } from 'node:crypto';
import type { Model, ModelReply, ModelRequest } from './types.ts';

export const STUB_MODEL_ID = 'stub-1';

export type StubScript = (req: ModelRequest, call: number) => string;

/** Proposes nothing: `{"actions":[]}`. */
export const idleScript: StubScript = () => JSON.stringify({ actions: [] });

/** Replays fixed replies in order, then proposes nothing. */
export function replay(replies: readonly unknown[]): StubScript {
  return (_req, call) => (call < replies.length ? (typeof replies[call] === 'string' ? (replies[call] as string) : JSON.stringify(replies[call])) : JSON.stringify({ actions: [] }));
}

export function stubModel(script: StubScript = idleScript, model = STUB_MODEL_ID): Model & { calls: ModelRequest[] } {
  const calls: ModelRequest[] = [];
  return {
    provider: 'stub',
    model,
    calls,
    async complete(req: ModelRequest): Promise<ModelReply> {
      const n = calls.length;
      calls.push(req);
      return { provider: 'stub', model, text: script(req, n) };
    },
  };
}

/** A stable fingerprint of a request (used by the stub's own tests). */
export function requestFingerprint(req: ModelRequest): string {
  return createHash('sha256').update(JSON.stringify(req)).digest('hex');
}
