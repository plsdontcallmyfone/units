// One adapter for every OpenAI-compatible chat completions API (OpenAI, DeepSeek, Qwen, Kimi, GLM,
// MiniMax, Llama hosts, ...): the provider is a base URL, a key and a model id.
import type { Secret } from '../env.ts';
import { errorSnippet, ModelError, type Fetch, type Model, type ModelReply, type ModelRequest } from './types.ts';

export interface OpenAiCompatibleOptions { id: string; apiKey: Secret; baseUrl: string; model: string; fetch?: Fetch; timeoutMs?: number }

export function openAiCompatibleModel(o: OpenAiCompatibleOptions): Model {
  const base = o.baseUrl.replace(/\/+$/, '');
  const f: Fetch = o.fetch ?? (globalThis.fetch as unknown as Fetch);
  return {
    provider: o.id,
    model: o.model,
    async complete(req: ModelRequest): Promise<ModelReply> {
      const res = await f(`${base}/chat/completions`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', authorization: `Bearer ${o.apiKey.reveal()}` },
        body: JSON.stringify({
          model: o.model, max_tokens: req.maxTokens, temperature: req.temperature,
          messages: [{ role: 'system', content: req.system }, ...req.messages],
        }),
        signal: AbortSignal.timeout(o.timeoutMs ?? 60_000),
      });
      const body = await res.text();
      if (!res.ok) throw new ModelError(o.id, res.status, errorSnippet(body));
      let j: { model?: string; choices?: { message?: { content?: string | null } }[] };
      try { j = JSON.parse(body); } catch { throw new ModelError(o.id, res.status, 'reply is not JSON'); }
      return { provider: o.id, model: j.model ?? o.model, text: j.choices?.[0]?.message?.content ?? '' };
    },
  };
}
