/** Server-side reads from the backend (06: the browser talks only to the site's /api routes; server
 * components read the backend directly). A failed read is reported, never replaced by a figure. */
import 'server-only';

export const API_URL = process.env.API_URL ?? 'http://127.0.0.1:9961';

export type Read<T> = { ok: true; data: T } | { ok: false; error: string };

export async function read<T>(path: string): Promise<Read<T>> {
  try {
    const r = await fetch(API_URL + path, { cache: 'no-store', signal: AbortSignal.timeout(8000) });
    const body = (await r.json()) as unknown;
    // The API's own error sentences are written for people; anything else stays in the log (A-8).
    if (!r.ok) return { ok: false, error: (body as { error?: string })?.error ?? 'The backend could not read this.' };
    return { ok: true, data: body as T };
  } catch (e) {
    console.error('[read]', path, e instanceof Error ? e.message : String(e));
    return { ok: false, error: 'The backend is not reachable.' };
  }
}
