import Link from 'next/link';
import type { BattleEvent, HookwarsStatus, Page, WarMap } from '@hookwars/shared';
import { read } from '@/lib/api';
import { DASH, ago, int, short } from '@/lib/format';
import { Empty, Panel, ReadFailed } from '@/components/ui';

type LaunchRow = Record<string, unknown>;
type Params = { q?: string; sort?: string; state?: string; item?: string };
type State = 'siege' | 'war' | 'quiet';

/** One token on the board: the launch row joined with what the map and the feed say about it. */
type Card = {
  mint: string; name: string; symbol: string; image: string | null; slot: number | null;
  chest: string | null; raids: number; lastTs: number | null; state: State; item: string | null; itemKind: string | null;
};

const STATE_LABEL: Record<State, string> = { siege: 'UNDER SIEGE', war: 'AT WAR', quiet: 'QUIET' };
const SORTS: [string, string][] = [['', 'Top'], ['new', 'New'], ['chest', 'Biggest chest'], ['war', 'At war first']];
const STATES: [string, string][] = [['', 'All'], ['war', 'At war'], ['siege', 'Under siege'], ['quiet', 'Quiet']];

function href(params: Params, key: keyof Params, value: string): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) if (v && k !== key) p.set(k, v);
  if (value) p.set(key, value);
  return p.size ? `/?${p}` : '/';
}

function Segment({ label, values, selected, params, param }: { label: string; values: [string, string][]; selected: string; params: Params; param: keyof Params }) {
  return <div className="seg" role="tablist" aria-label={label}>{values.map(([v, name]) => <Link key={name} role="tab" aria-selected={selected === v} className={selected === v ? 'on' : ''} href={href(params, param, v)}>{name}</Link>)}</div>;
}

function TokenCard({ c, rank }: { c: Card; rank: number }) {
  return (
    <Link className="card" href={`/t/${c.mint}`}>
      <div className="card-media">
        {c.image ? <img src={c.image} alt="" loading="lazy" /> : <span className="glyph">{c.symbol.slice(0, 4)}</span>}
        <span className="card-rank">#{String(rank).padStart(2, '0')}</span>
        <span className={`card-state ${c.state}`}>{STATE_LABEL[c.state]}</span>
      </div>
      <div className="card-body">
        <div className="card-title"><span className="card-name">{c.name}</span><span className="card-symbol">${c.symbol}</span></div>
        <dl className="card-stats">
          <div><dt>CHEST SOL</dt><dd>{c.chest ? (Number(c.chest) / 1e9).toLocaleString('en-US', { maximumFractionDigits: 2 }) : DASH}</dd></div>
          <div><dt>RAIDS</dt><dd>{int(c.raids)}</dd></div>
          <div><dt>LAST</dt><dd>{c.lastTs ? ago(c.lastTs) : DASH}</dd></div>
        </dl>
        <div className="card-item">
          <span className="card-item-name">{c.item ?? 'No item equipped'}</span>
          <span className="card-item-kind">{c.itemKind ?? 'empty slots'}</span>
        </div>
      </div>
    </Link>
  );
}

export default async function Projects({ searchParams }: { searchParams: Promise<Params> }) {
  const params = await searchParams;
  const [launches, status, feed, map] = await Promise.all([
    read<Page<LaunchRow>>('/v1/launches'), read<HookwarsStatus>('/v1/status'), read<Page<BattleEvent>>('/v1/feed'), read<WarMap>('/v1/map'),
  ]);
  const nodes = new Map(map.ok ? map.data.nodes.map((n) => [n.mint, n]) : []);
  const events = feed.ok ? feed.data.items : [];
  const warring = new Set(map.ok ? map.data.edges.flatMap((e) => [e.from, e.to]) : []);
  const cards: Card[] = launches.ok ? launches.data.items.map((l) => {
    const mint = String(l.mint);
    const node = nodes.get(mint);
    const mine = events.filter((e) => e.mint === mint || e.otherMint === mint);
    const equip = typeof l.item === 'string' ? l.item : null;
    return {
      mint, name: String(l.name ?? l.symbol ?? short(mint)), symbol: String(l.symbol ?? short(mint)), image: typeof l.image === 'string' ? l.image : null,
      slot: typeof l.slot === 'number' ? l.slot : null, chest: node?.chest ?? null,
      raids: mine.filter((e) => e.kind === 'raid').length, lastTs: mine[0]?.ts ?? null,
      state: node?.underSiege ? 'siege' : warring.has(mint) ? 'war' : 'quiet', item: equip, itemKind: typeof l.itemKind === 'string' ? l.itemKind : null,
    };
  }) : [];

  const q = params.q?.toLowerCase().trim();
  let visible = cards.filter((c) => (!params.state || c.state === params.state) && (!params.item || c.itemKind === params.item) && (!q || `${c.name} ${c.symbol} ${c.mint}`.toLowerCase().includes(q)));
  if (params.sort === 'new') visible = [...visible].sort((a, b) => (b.slot ?? 0) - (a.slot ?? 0));
  if (params.sort === 'chest') visible = [...visible].sort((a, b) => Number(b.chest ?? 0) - Number(a.chest ?? 0));
  if (params.sort === 'war') visible = [...visible].sort((a, b) => Number(a.state === 'quiet') - Number(b.state === 'quiet') || b.raids - a.raids);
  if (!params.sort) visible = [...visible].sort((a, b) => b.raids - a.raids || Number(b.chest ?? 0) - Number(a.chest ?? 0));

  const kinds = new Map<string, number>();
  for (const c of cards) if (c.itemKind) kinds.set(c.itemKind, (kinds.get(c.itemKind) ?? 0) + 1);
  const programs = status.ok ? status.data.programs : [];
  const deployed = programs.filter((p) => p.deployed).length;

  return (
    <>
      <header className="hero-box">
        <div>
          <div className="eyebrow">Projects</div>
          <h1>Tokens at war</h1>
          <p className="lede">Every token here runs hooks that are owned items. Its holders vote what fills each slot, aim raids at rivals, and fund a war chest from their own fees.</p>
        </div>
        <dl className="hero-stats">
          <div><dt>Tokens</dt><dd>{launches.ok ? int(cards.length) : DASH}</dd></div>
          <div><dt>At war</dt><dd>{launches.ok ? int(cards.filter((c) => c.state !== 'quiet').length) : DASH}</dd></div>
          <div><dt>Under siege</dt><dd>{launches.ok ? int(cards.filter((c) => c.state === 'siege').length) : DASH}</dd></div>
          <div><dt>Programs</dt><dd>{status.ok ? `${deployed}/${programs.length}` : DASH}</dd></div>
        </dl>
      </header>
      <div className="directory">
        <aside className="rail" aria-label="Filter by equipped item">
          <div className="rail-title">Equipped</div>
          <Link className={`rail-item ${params.item ? '' : 'on'}`} href={href(params, 'item', '')}><span>All tokens</span><span className="rail-count">{cards.length}</span></Link>
          {[...kinds.entries()].sort((a, b) => b[1] - a[1]).map(([k, n]) => <Link key={k} className={`rail-item ${params.item === k ? 'on' : ''}`} href={href(params, 'item', k)}><span>{k}</span><span className="rail-count">{n}</span></Link>)}
        </aside>
        <section>
          <div className="toolbar">
            <form action="/" className="search"><input name="q" aria-label="Search tokens" placeholder="Search name, ticker or mint" defaultValue={params.q ?? ''} />{params.item ? <input type="hidden" name="item" value={params.item} /> : null}</form>
            <Segment label="Sort" values={SORTS} selected={params.sort ?? ''} params={params} param="sort" />
            <Segment label="State" values={STATES} selected={params.state ?? ''} params={params} param="state" />
          </div>
          {!launches.ok ? <ReadFailed what="launches" error={launches.error} /> : cards.length === 0 ? (
            <Empty title="No launches on this cluster yet" what="A token appears here once its create_launch lands and the indexer reads its LaunchCreated event. Each card shows its chest, its raids and the item it runs." next={<>Start one from <Link href="/launch">Launch</Link>. It needs the launchpad, armory and items programs on this cluster.</>} />
          ) : (
            <>
              <div className="result">Showing {visible.length} of {cards.length} tokens</div>
              <div className="card-grid">{visible.map((c, i) => <TokenCard key={c.mint} c={c} rank={i + 1} />)}</div>
            </>
          )}
          <div className="grid cols-2" style={{ marginTop: 'var(--s6)' }}>
            <Panel title="Indexer" meta="one cursor per program" flush>
              {!status.ok ? <ReadFailed what="the indexer" error={status.error} /> : status.data.indexer.length === 0 ? <Empty title="The indexer has not run yet" what="Each program gets a cursor on the indexer's first pass." /> : (
                <table>
                  <thead><tr><th>Program</th><th className="num">Last slot</th><th className="num">Moved</th></tr></thead>
                  <tbody>{status.data.indexer.map((c) => <tr key={c.program}><td style={{ textTransform: 'capitalize' }}>{c.program}</td><td className="num">{c.lastSlot ? int(c.lastSlot) : DASH}</td><td className="num faint">{c.updatedAt ? ago(c.updatedAt) : DASH}</td></tr>)}</tbody>
                </table>
              )}
            </Panel>
            <Panel title="Network" meta={status.ok ? (status.data.database ? 'indexer database up' : 'database down') : undefined} flush>
              {!status.ok ? <ReadFailed what="the network" error={status.error} /> : (
                <div className="rows">
                  {programs.map((p) => (
                    <div className="row" key={p.name}>
                      <span className={`dot ${p.deployed ? 'ok' : 'warn'}`} aria-hidden />
                      <div style={{ minWidth: 0 }}><div style={{ fontWeight: 550, textTransform: 'capitalize' }}>{p.name}</div><div className="faint" title={p.address}>{short(p.address, 6)}</div></div>
                      <span className={`chip ${p.deployed ? 'ok' : 'warn'}`}>{p.deployed ? 'deployed' : 'not deployed'}</span>
                    </div>
                  ))}
                </div>
              )}
            </Panel>
          </div>
        </section>
      </div>
    </>
  );
}
