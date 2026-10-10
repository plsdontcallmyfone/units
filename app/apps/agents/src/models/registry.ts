// Provider registry: an agent names a provider id and, optionally, a model id; the registry builds
// the adapter from keys and URLs read from the environment at start.
import type { RuntimeEnv } from '../env.ts';
import { envId } from '../env.ts';
import { anthropicModel, DEFAULT_CLAUDE_MODEL } from './anthropic.ts';
import { openAiCompatibleModel } from './openai.ts';
import { stubModel, type StubScript } from './stub.ts';
import type { Fetch, Model } from './types.ts';

export interface ModelSpec {
  /** `anthropic`, `stub`, or any OpenAI-compatible provider id (`openai`, `deepseek`, `qwen`, `kimi`, `glm`, `minimax`, `llama`, ...). */
  provider: string;
  /** Model id; falls back to `UNITS_<ID>_MODEL` (and for Anthropic to `UNITS_ANTHROPIC_MODEL`, then the default Claude id). */
  model?: string;
}

/** Only OpenAI's own base URL is built in; every other OpenAI-compatible provider sets `UNITS_<ID>_BASE_URL`. */
const BUILT_IN_BASE_URLS: Record<string, string> = { openai: 'https://api.openai.com/v1' };

export class ModelConfigError extends Error {}

export interface RegistryOptions { fetch?: Fetch; stubScript?: StubScript }

/** What a spec needs from the environment (for `units-agents env` and the README). */
export function requiredEnv(spec: ModelSpec): string[] {
  if (spec.provider === 'stub') return [];
  const e = envId(spec.provider);
  const out = [`UNITS_${e}_API_KEY`];
  if (spec.provider !== 'anthropic' && !BUILT_IN_BASE_URLS[spec.provider]) out.push(`UNITS_${e}_BASE_URL`);
  if (!spec.model && spec.provider !== 'anthropic') out.push(`UNITS_${e}_MODEL`);
  return out;
}

export function resolveModel(spec: ModelSpec, env: RuntimeEnv, opts: RegistryOptions = {}): Model {
  if (spec.provider === 'stub') return stubModel(opts.stubScript, spec.model);
  const p = env.provider(spec.provider);
  const e = envId(spec.provider);
  if (!p.apiKey) throw new ModelConfigError(`provider "${spec.provider}" has no key: set UNITS_${e}_API_KEY`);
  if (spec.provider === 'anthropic') {
    const model = spec.model ?? p.model ?? env.anthropicModel ?? DEFAULT_CLAUDE_MODEL;
    return anthropicModel({ apiKey: p.apiKey, model, ...(p.baseUrl ? { baseUrl: p.baseUrl } : {}), ...(opts.fetch ? { fetch: opts.fetch } : {}) });
  }
  const baseUrl = p.baseUrl ?? BUILT_IN_BASE_URLS[spec.provider];
  if (!baseUrl) throw new ModelConfigError(`provider "${spec.provider}" has no base URL: set UNITS_${e}_BASE_URL`);
  if (!/^https:\/\//.test(baseUrl) && !/^http:\/\/(127\.0\.0\.1|localhost)(:\d+)?(\/|$)/.test(baseUrl)) throw new ModelConfigError(`provider "${spec.provider}": the base URL must be https (or a local http address)`);
  const model = spec.model ?? p.model;
  if (!model) throw new ModelConfigError(`provider "${spec.provider}" has no model id: set it in the agent config or UNITS_${e}_MODEL`);
  return openAiCompatibleModel({ id: spec.provider, apiKey: p.apiKey, baseUrl, model, ...(opts.fetch ? { fetch: opts.fetch } : {}) });
}
