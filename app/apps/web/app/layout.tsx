import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { GeistSans } from 'geist/font/sans';
import './globals.css';
import { Nav } from '@/components/nav';
import { read } from '@/lib/api';
import type { HookwarsStatus } from '@hookwars/shared';

export const metadata: Metadata = {
  title: 'units',
  description: 'Tokens whose hooks are owned items, and that react to each other: raids, sieges, treaties. Spot only, on Solana.',
};

export const dynamic = 'force-dynamic';

async function ClusterBadge() {
  const s = await read<HookwarsStatus>('/v1/status');
  if (!s.ok) return <span className="cluster"><span className="dot bad" />Backend unreachable</span>;
  const deployed = s.data.programs.filter((p) => p.deployed).length;
  const total = s.data.programs.length;
  const name = s.data.cluster.includes('devnet') ? 'Devnet' : s.data.cluster.includes('mainnet') ? 'Mainnet' : 'Cluster';
  const cls = !s.data.rpcReachable ? 'bad' : deployed === total && total > 0 ? 'ok' : 'warn';
  return (
    <span className="cluster" title={`${deployed} of ${total} units programs deployed`}>
      <span className={`dot ${cls}`} />{name}{total ? ` · ${deployed}/${total} programs` : ''}
    </span>
  );
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={GeistSans.variable}>
      <body>
        <div className="topbar">
          <div className="topbar-inner">
            <a href="/" className="wordmark" aria-label="units home">units</a>
            <Nav />
            <ClusterBadge />
          </div>
        </div>
        <main className="shell">{children}</main>
      </body>
    </html>
  );
}
