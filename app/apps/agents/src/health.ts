// Health endpoint: GET /healthz answers what each agent last did (no keys, no prompts, no URLs with
// credentials). The port is the first free one in 9970 to 9979 unless UNITS_HEALTH_PORT names one.
import { createServer, type Server } from 'node:http';
import { createServer as netServer } from 'node:net';

export const HEALTH_PORTS = [9970, 9971, 9972, 9973, 9974, 9975, 9976, 9977, 9978, 9979];

export interface AgentHealth { name: string; passport: string | null; lastTickAt: number | null; directiveSeq: number | null; halted: string | null; lastError: string | null; ticks: number; sends: boolean; provider: string; model: string }

export function portFree(port: number, host = '127.0.0.1'): Promise<boolean> {
  return new Promise((ok) => {
    const s = netServer();
    s.once('error', () => ok(false));
    s.once('listening', () => s.close(() => ok(true)));
    s.listen(port, host);
  });
}

export async function pickPort(wanted: number | null): Promise<number> {
  if (wanted !== null) {
    if (!HEALTH_PORTS.includes(wanted)) throw new Error(`UNITS_HEALTH_PORT must be one of ${HEALTH_PORTS[0]} to ${HEALTH_PORTS.at(-1)}`);
    if (!(await portFree(wanted))) throw new Error(`port ${wanted} is in use`);
    return wanted;
  }
  for (const p of HEALTH_PORTS) if (await portFree(p)) return p;
  throw new Error('no free port in 9970 to 9979');
}

export function startHealth(port: number, read: () => AgentHealth[], host = '127.0.0.1'): Promise<Server> {
  const server = createServer((req, res) => {
    if (req.method === 'GET' && (req.url === '/healthz' || req.url === '/')) {
      const agents = read();
      const ok = agents.every((a) => a.lastError === null);
      res.writeHead(ok ? 200 : 503, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ ok, agents }));
      return;
    }
    res.writeHead(404, { 'content-type': 'application/json' });
    res.end('{"error":"not found"}');
  });
  return new Promise((ok) => server.listen(port, host, () => ok(server)));
}
