import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { SiteHeader, type TickerItem } from '@/components/site-header';
import { Cursor } from '@/components/cursor';
import './globals.css';
import '../components/site-header.css';
import type { BattleEvent, Page } from '@hookwars/shared';
import { ago, short } from '@/lib/format';
import { read } from '@/lib/api';
import type { HookwarsStatus } from '@hookwars/shared';
import { MOCK } from '@/lib/mock';

export const metadata: Metadata = {
  title: 'units',
  description: 'Tokens whose hooks are owned items, and that react to each other: raids, sieges, treaties. Spot only, on Solana.',
};

export const dynamic = 'force-dynamic';

const KIND: Record<string, string> = { raid: 'raided', siege: 'besieged', counter_strike: 'counter-struck', treaty_on: 'signed with', proposal: 'proposed on', equip: 'equipped on', forge: 'forged on', loot: 'looted on', bounty: 'claimed on', raze: 'razed', return: 'returned to' };

async function headerData(): Promise<{ ticker: TickerItem[]; cluster: { name: string; cls: string; title: string } }> {
  const [s, f] = await Promise.all([read<HookwarsStatus>('/v1/status'), read<Page<BattleEvent>>('/v1/feed')]);
  const ticker: TickerItem[] = f.ok ? f.data.items.slice(0, 14).map((e) => ({ who: e.actor ? short(e.actor, 4) : short(e.mint, 4), verb: KIND[e.kind] ?? e.kind, what: e.otherMint ? short(e.otherMint, 4) : short(e.mint, 4), tool: `${e.detail.template ?? e.kind} · ${ago(e.ts)}` })) : [];
  if (!s.ok) return { ticker, cluster: { name: 'Backend down', cls: 'bad', title: s.error } };
  const deployed = s.data.programs.filter((p) => p.deployed).length, total = s.data.programs.length;
  const name = s.data.cluster.includes('devnet') ? 'Devnet' : s.data.cluster.includes('mainnet') ? 'Mainnet' : 'Cluster';
  return { ticker, cluster: { name, cls: !s.data.rpcReachable ? 'bad' : deployed === total && total > 0 ? 'ok' : 'warn', title: `${deployed} of ${total} units programs deployed` } };
}

export default async function RootLayout({ children }: { children: ReactNode }) {
  const h = await headerData();
  return (
    <html lang="en">
      <body>
        <Cursor />
        <SiteHeader ticker={h.ticker} cluster={h.cluster} mock={MOCK} />
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
