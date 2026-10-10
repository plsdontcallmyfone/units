import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import './globals.css';
import { Nav, NavSheet } from '@/components/nav';
import { read } from '@/lib/api';
import type { HookwarsStatus } from '@hookwars/shared';
import { MOCK } from '@/lib/mock';

export const metadata: Metadata = {
  title: 'units',
  description: 'Tokens whose hooks are owned items, and that react to each other: raids, sieges, treaties. Spot only, on Solana.',
};

export const dynamic = 'force-dynamic';

async function ClusterBadge() {
  const s = await read<HookwarsStatus>('/v1/status');
  if (!s.ok) return <span className="nav-clock"><span className="dot bad" />BACKEND UNREACHABLE</span>;
  const deployed = s.data.programs.filter((p) => p.deployed).length;
  const total = s.data.programs.length;
  const name = s.data.cluster.includes('devnet') ? 'DEVNET' : s.data.cluster.includes('mainnet') ? 'MAINNET' : 'CLUSTER';
  const cls = !s.data.rpcReachable ? 'bad' : deployed === total && total > 0 ? 'ok' : 'warn';
  return (
    <span className="nav-clock" title={`${deployed} of ${total} units programs deployed`}>
      <span className={`dot ${cls}`} />{name}{total ? ` ${deployed}/${total}` : ''}
    </span>
  );
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header className="nav">
          <div className="wrap nav-row">
            <a href="/" className="nav-logo" aria-label="units home">units</a>
            <Nav />
            <div className="nav-end">
              {MOCK ? <span className="chip warn" title="MOCK_DATA=1: the site shows invented tokens and events, not the chain">Demo data</span> : null}
              <ClusterBadge />
              <a className="btn sm primary" href="/launch">Launch</a>
            </div>
          </div>
          <NavSheet />
        </header>
        <main className="wrap shell">{children}</main>
        <footer className="foot">
          <div className="wrap foot-grid">
            <div className="foot-brand">
              <div className="nav-logo">units</div>
              <p>Tokens whose hooks are owned items. Holders vote what fills each slot, aim raids at rivals and fund a war chest from their own fees. Spot only.</p>
            </div>
            <div className="foot-col"><div className="label">PLATFORM</div><a href="/launch">Launch</a><a href="/">Projects</a><a href="/war">War room</a><a href="/armory">Armory</a></div>
            <div className="foot-col"><div className="label">DOCS</div><a href="/docs">How units works</a><a href="/docs/protocol#templates">Templates</a><a href="/docs/protocol#parameters">Parameters</a></div>
            <div className="foot-col"><div className="label">NETWORK</div><span>Solana</span><span>Spot only</span><span>Bordrless standard</span></div>
          </div>
        </footer>
      </body>
    </html>
  );
}
