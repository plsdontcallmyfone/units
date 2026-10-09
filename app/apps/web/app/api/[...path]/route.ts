/** The browser talks only to the site's own /api routes; this proxies them to the backend with no
 * user key (06 section 1 rule 5, upstream architecture "Off chain"). */
import { API_URL } from '@/lib/api';

async function forward(req: Request, ctx: { params: Promise<{ path: string[] }> }): Promise<Response> {
  const { path } = await ctx.params;
  const url = new URL(req.url);
  const target = `${API_URL}/${path.map(encodeURIComponent).join('/')}${url.search}`;
  try {
    const r = await fetch(target, {
      method: req.method,
      headers: { 'content-type': req.headers.get('content-type') ?? 'application/json' },
      body: req.method === 'GET' || req.method === 'HEAD' ? undefined : await req.text(),
      cache: 'no-store',
    });
    return new Response(r.body, { status: r.status, headers: { 'content-type': r.headers.get('content-type') ?? 'application/json' } });
  } catch (e) {
    return Response.json({ error: `The backend did not answer: ${e instanceof Error ? e.message : String(e)}` }, { status: 502 });
  }
}

export const GET = forward;
export const POST = forward;
export const dynamic = 'force-dynamic';
