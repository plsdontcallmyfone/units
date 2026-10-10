// Native Anthropic Messages API adapter. The key is sent only in the `x-api-key` header to the
// configured base URL.
import type { Secret } from '../env.ts';
import { errorSnippet, ModelError, type Fetch, type Model, type ModelReply, type ModelRequest } from './types.ts';

export const ANTHROPIC_BASE_URL = 'https://api.anthropic.com';
export const ANTHROPIC_VERSION = '2023-06-01';
/** The default Claude model id when neither the agent config nor `UNITS_ANTHROPIC_MODEL` names one. */
export const DEFAULT_CLAUDE_MODEL = 'claude-opus-5-5';

export interface AnthropicOptions { apiKey: Secret; model: string; baseUrl?: string; fetch?: Fetch; timeoutMs?: number }

export function anthropicModel(o: AnthropicOptions): Model {
  const base = (o.baseUrl ?? ANTHROPIC_BASE_URL).replace(/\/+$/, '');
  const f: Fetch = o.fetch ?? (globalThis.fetch as unknown as Fetch);
  return {
    provider: 'anthropic',
    model: o.model,
    async complete(req: ModelRequest): Promise<ModelReply> {
      const res = await f(`${base}/v1/messages`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-api-key': o.apiKey.reveal(), 'anthropic-version': ANTHROPIC_VERSION },
        body: JSON.stringify({ model: o.model, max_tokens: req.maxTokens, temperature: req.temperature, system: req.system, messages: req.messages }),
        signal: AbortSignal.timeout(o.timeoutMs ?? 60_000),
      });
      const body = await res.text();
      if (!res.ok) throw new ModelError('anthropic', res.status, errorSnippet(body));
      let j: { model?: string; content?: { type: string; text?: string }[] };
      try { j = JSON.parse(body); } catch { throw new ModelError('anthropic', res.status, 'reply is not JSON'); }
      const text = (j.content ?? []).filter((c) => c.type === 'text').map((c) => c.text ?? '').join('');
      return { provider: 'anthropic', model: j.model ?? o.model, text };
    },
  };
}
