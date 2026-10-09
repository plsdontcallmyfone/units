// Changed by Hookwars: path allow-list, body cap, generic errors (app audit A-3, A-8, A-11).
/** The browser talks only to the site's own /api routes; this proxies them to the backend with no
 * user key (06 section 1 rule 5, upstream architecture "Off chain"). Only `v1/` paths are forwarded,
 * bodies are capped, and a failure answers a generic message with a request id while the detail is
 * logged here (audit A-3, A-8, A-11). */
import { API_URL } from '@/lib/api';
import { BodyTooLarge, forwardPath, readCapped } from '@/lib/proxy';

async function forward(req: Request, ctx: { params: Promise<{ path: string[] }> }): Promise<Response> {
  const { path } = await ctx.params;
  const rel = forwardPath(path);
  if (!rel) return Response.json({ error: 'No such route.' }, { status: 404 });
  const url = new URL(req.url);
  let body: string | undefined;
  try {
    body = await readCapped(req);
  } catch (e) {
    if (e instanceof BodyTooLarge) return Response.json({ error: 'The request body is too large.' }, { status: 413 });
    throw e;
  }
  try {
    const r = await fetch(`${API_URL}/${rel}${url.search}`, {
      method: req.method,
      headers: {
        'content-type': req.headers.get('content-type') ?? 'application/json',
        // The client's address, for the API's rate limit (the site is the only caller).
        'x-forwarded-for': req.headers.get('x-forwarded-for') ?? '',
      },
      body,
      cache: 'no-store',
    });
    return new Response(r.body, { status: r.status, headers: { 'content-type': r.headers.get('content-type') ?? 'application/json' } });
  } catch (e) {
    const id = crypto.randomUUID();
    console.error(`[proxy ${id}]`, e instanceof Error ? e.message : String(e));
    return Response.json({ error: 'The backend did not answer.', requestId: id }, { status: 502 });
  }
}

export const GET = forward;
export const POST = forward;
export const dynamic = 'force-dynamic';
