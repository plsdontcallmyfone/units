/** The /api proxy's checks, kept pure so they are unit tested (audit A-3, A-8, A-11). */

/** The largest request body the site forwards (the API holds the same cap). */
export const MAX_BODY_BYTES = 64 * 1024;

/** The API path to forward, or null: only `v1/...`, and no empty, `.` or `..` segment. */
export function forwardPath(segments: string[]): string | null {
  if (segments.length < 2 || segments[0] !== 'v1') return null;
  for (const s of segments) {
    if (s === '' || s === '.' || s === '..' || s.includes('/') || s.includes('\\')) return null;
  }
  return segments.map(encodeURIComponent).join('/');
}

export class BodyTooLarge extends Error {}

/** Reads a request body up to `max` bytes; throws `BodyTooLarge` past it, checking the declared
 * length first so nothing is buffered for a body that says it is too large. */
export async function readCapped(req: Request, max = MAX_BODY_BYTES): Promise<string | undefined> {
  if (req.method === 'GET' || req.method === 'HEAD') return undefined;
  const declared = Number(req.headers.get('content-length') ?? '0');
  if (Number.isFinite(declared) && declared > max) throw new BodyTooLarge();
  if (!req.body) return '';
  const reader = req.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > max) { await reader.cancel(); throw new BodyTooLarge(); }
    chunks.push(value);
  }
  const all = new Uint8Array(total);
  let off = 0;
  for (const c of chunks) { all.set(c, off); off += c.byteLength; }
  return new TextDecoder().decode(all);
}
