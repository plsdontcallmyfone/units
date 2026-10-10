// Changed by Hookwars: new file (explorer v2). A program: its interface (instructions, events),
// what the indexer counted, and its recent transactions on this cluster.
import { read } from '@/lib/api';
import { int, short } from '@/lib/format';
import type { ProgramPage } from '@/lib/explorer';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { SearchBox, Signatures } from '@/components/explorer-ui';
import '../../explorer/explorer.css';

export default async function Program({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const r = await read<ProgramPage>(`/v1/explorer/program/${encodeURIComponent(id)}?limit=40`);
  if (!r.ok) {
    return (<><Head eyebrow="Program" title={short(id, 6)} /><Panel title="Search"><SearchBox /></Panel><div style={{ height: 16 }} /><Panel title="Program"><ReadFailed what="this program" error={r.error} /></Panel></>);
  }
  const p = r.data;
  return (
    <>
      <Head eyebrow="Program" title={p.title ?? short(id, 6)} lede={<span className="x-val">{id}</span>}
        right={p.deployed ? <span className="chip ok">deployed here</span> : <span className="chip warn">not on this cluster</span>} />
      <div className="stats">
        <Stat label="Instructions" value={int(p.instructions.length)} />
        <Stat label="Events" value={int(p.events.length)} />
        <Stat label="Indexed events" value={p.indexedCounts ? int(p.indexedCounts.reduce((s, c) => s + c.n, 0)) : 'not indexed'} />
      </div>
      <Panel title="Recent transactions" meta="newest first, from the cluster" flush>
        <Signatures list={p.recent} empty={<Empty title="No transactions" what={p.deployed ? 'Nothing has called this program on this cluster yet.' : 'The program is not deployed on this cluster.'} />} />
      </Panel>
      <div style={{ height: 16 }} />
      <div className="x-grid2">
        <Panel title="Indexed events by name" flush>
          {!p.indexedCounts ? <Empty title="History needs the indexer" what="Counts come from the indexer, which this backend is not connected to." />
            : p.indexedCounts.length === 0 ? <Empty title="No events indexed" what="The indexer holds no event of this program yet." />
            : <div className="x-scroll"><table><thead><tr><th>Event</th><th className="r">Count</th><th className="r">Last slot</th></tr></thead>
              <tbody>{p.indexedCounts.map((c) => <tr key={c.name}><td>{c.name}</td><td className="r tabular">{int(c.n)}</td><td className="r tabular">{int(c.last_slot)}</td></tr>)}</tbody></table></div>}
        </Panel>
        <Panel title="Interface">
          {p.instructions.length === 0 ? <Empty title="No interface" what="The explorer has no interface for this program." /> : (
            <>
              <div className="label" style={{ marginBottom: 8 }}>Instructions</div>
              <div className="x-chips">{p.instructions.map((n) => <span key={n} className="chip">{n}</span>)}</div>
              <div className="sep" />
              <div className="label" style={{ marginBottom: 8 }}>Events</div>
              <div className="x-chips">{p.events.map((n) => <span key={n} className="chip">{n}</span>)}</div>
            </>
          )}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Search"><SearchBox /></Panel>
    </>
  );
}
