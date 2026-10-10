// Changed by Hookwars: new file. Turns an action form's typed values into a prepare body
// (components/action.tsx), kept pure so it is unit tested.

/** key: a base58 address; sol: typed in SOL, sent as lamports; amount: base units as typed; int;
 * bool; text; keys: comma separated addresses; ints: comma separated integers; json: a JSON value
 * (lists of modules or components). */
export type FieldKind = 'key' | 'sol' | 'amount' | 'int' | 'bool' | 'text' | 'keys' | 'ints' | 'json';
export type Field = { name: string; label: string; kind: FieldKind; hint?: string; optional?: boolean; choices?: [string, string][] };

const LAMPORTS = 1_000_000_000n;
export function solToLamports(s: string): string | null {
  const m = /^(\d+)(?:\.(\d{1,9}))?$/.exec(s.trim());
  return m ? (BigInt(m[1]!) * LAMPORTS + BigInt((m[2] ?? '').padEnd(9, '0'))).toString() : null;
}

/** The request body from typed values; a dotted name ("kind.to") nests. Returns an error sentence
 * for the first value that does not parse. */
export function bodyOf(fields: Field[], values: Record<string, string>, fixed: Record<string, unknown>): { body: Record<string, unknown> } | { error: string } {
  const body: Record<string, unknown> = structuredClone(fixed);
  for (const f of fields) {
    const raw = (values[f.name] ?? '').trim();
    if (raw === '' && f.kind !== 'bool') { if (f.optional) continue; return { error: `${f.label} is required.` }; }
    let v: unknown;
    switch (f.kind) {
      case 'sol': v = solToLamports(raw); if (v === null) return { error: `${f.label} must be a SOL amount, up to 9 decimals.` }; break;
      case 'amount': if (!/^\d+$/.test(raw)) return { error: `${f.label} must be a whole number of base units.` }; v = raw; break;
      case 'int': if (!/^\d+$/.test(raw)) return { error: `${f.label} must be a whole number.` }; v = Number(raw); break;
      case 'bool': v = raw === 'true'; break;
      case 'keys': v = raw.split(',').map((x) => x.trim()).filter(Boolean); break;
      case 'ints': { const xs = raw.split(',').map((x) => x.trim()).filter(Boolean); if (xs.some((x) => !/^\d+$/.test(x))) return { error: `${f.label} must be whole numbers separated by commas.` }; v = xs.map(Number); break; }
      case 'json': try { v = JSON.parse(raw); } catch { return { error: `${f.label} must be JSON.` }; } break;
      default: v = raw;
    }
    const path = f.name.split('.');
    let at = body;
    for (const p of path.slice(0, -1)) { at[p] = (at[p] as Record<string, unknown> | undefined) ?? {}; at = at[p] as Record<string, unknown>; }
    at[path[path.length - 1]!] = v;
  }
  return { body };
}
