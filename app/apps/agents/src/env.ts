// Everything the runtime reads from the environment, read once at start. Secrets are wrapped so
// they never reach a log line, a JSON dump, a memo or the health endpoint by accident.
import { inspect } from 'node:util';

/** A secret value: `reveal()` is the only way out; printing or serialising it shows `[redacted]`. */
export class Secret {
  readonly #value: string;
  constructor(value: string) { this.#value = value; }
  reveal(): string { return this.#value; }
  toString(): string { return '[redacted]'; }
  toJSON(): string { return '[redacted]'; }
  [inspect.custom](): string { return '[redacted]'; }
}

export type EnvMap = Readonly<Record<string, string | undefined>>;

/** Provider ids whose names are folded into env var names (`UNITS_<ID>_API_KEY`). */
export function envId(id: string): string { return id.toUpperCase().replace(/[^A-Z0-9]/g, '_'); }

export interface ProviderEnv {
  apiKey: Secret | null;
  baseUrl: string | null;
  model: string | null;
}

export interface RuntimeEnv {
  rpcUrl: string | null;
  apiUrl: string | null;
  lookupTable: string | null;
  healthPort: number | null;
  /** Default Claude model id for the native Anthropic adapter. */
  anthropicModel: string | null;
  provider(id: string): ProviderEnv;
  /** Every secret value read, for the log redactor. */
  secrets(): string[];
}

const LEGACY_KEYS: Record<string, string> = { anthropic: 'ANTHROPIC_API_KEY', openai: 'OPENAI_API_KEY' };

/**
 * Reads the environment once. Provider variables are `UNITS_<ID>_API_KEY`, `UNITS_<ID>_BASE_URL`
 * and `UNITS_<ID>_MODEL`; `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` are accepted as fallbacks.
 */
export function readEnv(env: EnvMap = process.env): RuntimeEnv {
  const snapshot = new Map<string, string>();
  for (const [k, v] of Object.entries(env)) if (typeof v === 'string' && /^(UNITS_|ANTHROPIC_API_KEY$|OPENAI_API_KEY$)/.test(k)) snapshot.set(k, v);
  const str = (k: string): string | null => { const v = snapshot.get(k)?.trim(); return v ? v : null; };
  const port = str('UNITS_HEALTH_PORT');
  const secretValues: string[] = [];
  for (const [k, v] of snapshot) if (/_API_KEY$/.test(k) && v.trim()) secretValues.push(v.trim());
  // Review 3 L-9: an RPC URL usually carries a provider key (in the query, the path or the user part).
  const rpc = snapshot.get('UNITS_RPC_URL')?.trim();
  if (rpc) {
    secretValues.push(rpc);
    try {
      const u = new URL(rpc);
      if (u.search.length > 1) secretValues.push(u.search.slice(1));
      if (u.password) secretValues.push(u.password);
      for (const seg of u.pathname.split('/')) if (seg.length >= 16) secretValues.push(seg);
    } catch { /* not a URL: the whole value is redacted already */ }
  }
  return {
    rpcUrl: str('UNITS_RPC_URL'),
    apiUrl: str('UNITS_API_URL'),
    lookupTable: str('UNITS_PROTOCOL_LOOKUP_TABLE'),
    healthPort: port === null ? null : Number(port),
    anthropicModel: str('UNITS_ANTHROPIC_MODEL'),
    provider(id: string): ProviderEnv {
      const e = envId(id);
      const key = str(`UNITS_${e}_API_KEY`) ?? (LEGACY_KEYS[id] ? str(LEGACY_KEYS[id]) : null);
      return { apiKey: key ? new Secret(key) : null, baseUrl: str(`UNITS_${e}_BASE_URL`), model: str(`UNITS_${e}_MODEL`) };
    },
    secrets: () => [...secretValues],
  };
}
