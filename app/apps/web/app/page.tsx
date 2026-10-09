import Link from 'next/link';
import type { HookwarsStatus, Page } from '@hookwars/shared';
import { read } from '@/lib/api';
import { DASH, ago, int, short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';

type LaunchRow = Record<string, unknown>;

export default async function Projects() {
  const [launches, status, feed] = await Promise.all([
    read<Page<LaunchRow>>('/v1/launches'),
    read<HookwarsStatus>('/v1/status'),
    read<Page<unknown>>('/v1/feed'),
  ]);
  const programs = status.ok ? status.data.programs : [];
  const deployed = programs.filter((p) => p.deployed).length;
  const lastSlot = status.ok ? Math.max(0, ...status.data.indexer.map((c) => c.lastSlot ?? 0)) : 0;
  const updated = status.ok ? Math.max(0, ...status.data.indexer.map((c) => c.updatedAt ?? 0)) : 0;
  return (
    <>
      <Head
        eyebrow="Projects"
        title="Tokens at war"
        lede="Every token here runs hooks that are owned items. Its holders vote what fills each slot, aim raids at rivals, and fund a war chest from their own fees."
        right={<Link className="btn primary" href="/launch">Launch a token</Link>}
      />
      <div className="stats">
        <Stat label="Launches indexed" value={launches.ok ? int(launches.data.items.length) : DASH} sub="LaunchCreated events" />
        <Stat label="Programs live" value={status.ok ? `${deployed} of ${programs.length}` : DASH} sub={status.ok ? (status.data.cluster.includes('devnet') ? 'devnet' : status.data.cluster) : 'backend unreachable'} />
        <Stat label="Battle events" value={feed.ok ? int(feed.data.items.length) : DASH} sub="raids, sieges, forges" />
        <Stat label="Indexed to slot" value={lastSlot ? int(lastSlot) : DASH} sub={updated ? `cursor moved ${ago(updated)}` : 'no cursor has moved yet'} />
        <Stat label="Chain slot" value={status.ok && status.data.slot ? int(status.data.slot) : DASH} sub={status.ok && status.data.rpcReachable ? 'RPC answering' : 'RPC not answering'} />
      </div>
      <div className="grid cols-main">
        <div className="grid">
        <Panel title="Launches" meta="newest first" flush>
          {!launches.ok ? <ReadFailed what="launches" error={launches.error} /> : launches.data.items.length === 0 ? (
            <Empty
              title="No launches on this cluster yet"
              what="A launch appears here once its create_launch lands and the indexer reads its LaunchCreated event. Each card will show its slots, war chest and whether it is under siege."
              next={<>Start one from <Link href="/launch">Launch</Link>. It needs the launchpad, armory and items programs on this cluster (see Network).</>}
            />
          ) : (
            <table>
              <thead><tr><th>Token</th><th>Mint</th><th>Creator</th></tr></thead>
              <tbody>
                {launches.data.items.map((l, i) => (
                  <tr key={i}><td><Link href={`/t/${String(l.mint)}`}>{String(l.symbol ?? l.name ?? short(String(l.mint)))}</Link></td><td className="addr">{short(String(l.mint))}</td><td className="addr">{short(String(l.creator ?? ''))}</td></tr>
                ))}
              </tbody>
            </table>
          )}
        </Panel>
        <Panel title="Indexer" meta="one signature cursor per program, oldest first" flush>
          {!status.ok ? <ReadFailed what="the indexer" error={status.error} /> : status.data.indexer.length === 0 ? (
            <Empty title="The indexer has not run yet" what="Each program gets a cursor on the indexer's first pass." />
          ) : (
            <table>
              <thead><tr><th>Program</th><th>Last signature</th><th className="num">Last slot</th><th className="num">Moved</th></tr></thead>
              <tbody>
                {status.data.indexer.map((c) => (
                  <tr key={c.program}><td style={{ textTransform: 'capitalize' }}>{c.program}</td><td className="addr">{c.cursor ? short(c.cursor, 8) : 'no transactions yet'}</td><td className="num">{c.lastSlot ? int(c.lastSlot) : DASH}</td><td className="num faint">{c.updatedAt ? ago(c.updatedAt) : DASH}</td></tr>
                ))}
              </tbody>
            </table>
          )}
        </Panel>
        </div>
        <div className="grid">
        <Panel title="Network" meta={status.ok ? (status.data.database ? 'indexer database up' : 'database down') : undefined} flush>
          {!status.ok ? <ReadFailed what="the network" error={status.error} /> : (
            <div className="rows">
              {programs.length === 0 ? <Empty title="No program info yet" what="The indexer fills this on its first pass." /> : programs.map((p) => (
                <div className="row" key={p.name}>
                  <span className={`dot ${p.deployed ? 'ok' : 'warn'}`} aria-hidden />
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontWeight: 550, textTransform: 'capitalize' }}>{p.name}</div>
                    <div className="faint" style={{ fontSize: 12 }} title={p.address}>{short(p.address, 6)}</div>
                  </div>
                  <span className={`chip ${p.deployed ? 'ok' : 'warn'}`}>{p.deployed ? 'deployed' : 'not deployed'}</span>
                </div>
              ))}
            </div>
          )}
        </Panel>
        </div>
      </div>
    </>
  );
}
