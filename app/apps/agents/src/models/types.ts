// The model layer's contract. A model only proposes: everything it returns is parsed and checked by
// deterministic code before anything is signed.

export interface ChatMessage { role: 'user' | 'assistant'; content: string }

export interface ModelRequest {
  system: string;
  messages: ChatMessage[];
  maxTokens: number;
  temperature: number;
}

export interface ModelReply {
  /** Registry id of the provider that answered (e.g. `anthropic`, `deepseek`, `stub`). */
  provider: string;
  /** The exact model id the provider ran. */
  model: string;
  text: string;
}

export interface Model {
  readonly provider: string;
  readonly model: string;
  complete(req: ModelRequest): Promise<ModelReply>;
}

/** `fetch` as the adapters use it (injected in tests; never logged). */
export type Fetch = (url: string, init: { method: string; headers: Record<string, string>; body: string; signal?: AbortSignal }) => Promise<{ ok: boolean; status: number; text(): Promise<string> }>;

export class ModelError extends Error {
  readonly provider: string;
  readonly status: number;
  constructor(provider: string, status: number, message: string) { super(`${provider}: ${message}`); this.provider = provider; this.status = status; }
}

/** An error body from a provider, trimmed so a log line stays small (it never contains our key). */
export function errorSnippet(body: string): string { return body.replace(/\s+/g, ' ').slice(0, 300); }
