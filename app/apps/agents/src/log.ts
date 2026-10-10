// Structured logs, one JSON object per line, with secrets removed: keys whose names look secret,
// any value equal to (or containing) a secret read at start, and 64-byte key arrays.

export type Level = 'debug' | 'info' | 'warn' | 'error';

export interface Logger {
  log(level: Level, event: string, fields?: Record<string, unknown>): void;
  child(fields: Record<string, unknown>): Logger;
}

const SECRET_KEY = /(secret|private|api[_-]?key|authorization|password|seed|keypair)/i;

export function redact(value: unknown, secrets: readonly string[], depth = 0): unknown {
  if (depth > 8) return '[deep]';
  if (typeof value === 'string') {
    let s = value;
    for (const sec of secrets) if (sec.length >= 8 && s.includes(sec)) s = s.split(sec).join('[redacted]');
    return s;
  }
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return value.length === 64 ? '[redacted]' : Buffer.from(value).toString('hex');
  if (Array.isArray(value)) {
    if (value.length === 64 && value.every((x) => Number.isInteger(x) && x >= 0 && x < 256)) return '[redacted]';
    return value.map((v) => redact(v, secrets, depth + 1));
  }
  if (value && typeof value === 'object') {
    if (typeof (value as { toBase58?: unknown }).toBase58 === 'function') return (value as { toBase58(): string }).toBase58();
    if (typeof (value as { toJSON?: unknown }).toJSON === 'function') return redact((value as { toJSON(): unknown }).toJSON(), secrets, depth + 1);
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) out[k] = SECRET_KEY.test(k) ? '[redacted]' : redact(v, secrets, depth + 1);
    return out;
  }
  return value;
}

export function makeLogger(opts: { secrets?: () => readonly string[]; write?: (line: string) => void; base?: Record<string, unknown>; minLevel?: Level } = {}): Logger {
  const order: Level[] = ['debug', 'info', 'warn', 'error'];
  const min = order.indexOf(opts.minLevel ?? 'info');
  const write = opts.write ?? ((line: string) => process.stdout.write(line + '\n'));
  const base = opts.base ?? {};
  return {
    log(level, event, fields = {}) {
      if (order.indexOf(level) < min) return;
      const rec = redact({ ts: new Date().toISOString(), level, event, ...base, ...fields }, opts.secrets?.() ?? []);
      write(JSON.stringify(rec));
    },
    child(fields) { return makeLogger({ ...opts, base: { ...base, ...fields } }); },
  };
}

/** A logger that keeps lines in memory (tests). */
export function memoryLogger(secrets: readonly string[] = []): Logger & { lines: string[] } {
  const lines: string[] = [];
  const l = makeLogger({ secrets: () => secrets, write: (s) => lines.push(s), minLevel: 'debug' });
  return Object.assign(l, { lines });
}
