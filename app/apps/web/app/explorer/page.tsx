// Changed by Hookwars: explorer v2. Search resolves on the server (signature, address, program,
// "template 7") and opens the page it names; the programs list leads to each program's activity.
import Link from 'next/link';
import { redirect } from 'next/navigation';
import { read } from '@/lib/api';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { SearchBox } from '@/components/explorer-ui';
import './explorer.css';

type Hit = { kind: string; value: string; href: string | null };
type Prog = { name: string; title: string; address: string };

export default async function Explorer({ searchParams }: { searchParams: Promise<{ q?: string }> }) {
  const { q } = await searchParams;
  let miss: string | null = null;
  if (q && q.trim()) {
    const hit = await read<Hit>(`/v1/explorer/search?q=${encodeURIComponent(q.trim())}`);
    if (hit.ok && hit.data.href) redirect(hit.data.href);
    miss = hit.ok ? 'That is not a signature, an address or "template" with a number.' : hit.error;
  }
  const programs = await read<Prog[]>('/v1/explorer/programs');
  return (
    <>
      <Head eyebrow="Explorer" title="Every units transaction, decoded" lede="Paste a signature to see each instruction and event in the order it ran: slot items on each transfer, pool items on swaps, raid marks, settlements and their split, war, market, craft and book actions, agent records and memos." />
      <Panel title="Search">
        <SearchBox q={q} />
        {miss ? <p className="reason" role="alert">{miss}</p> : null}
      </Panel>
      <div style={{ height: 16 }} />
      <Panel title="Programs" meta="recent activity on each program's page" flush>
        {!programs.ok ? <ReadFailed what="the program list" error={programs.error} /> : programs.data.length === 0 ? <Empty title="No programs" what="The API lists the programs it decodes." /> : (
          <div className="x-scroll">
            <table>
              <thead><tr><th>Program</th><th>Name</th><th>Address</th></tr></thead>
              <tbody>{programs.data.map((p) => (
                <tr key={p.address}>
                  <td><Link href={`/program/${p.address}`}>{p.title}</Link></td>
                  <td className="faint">{p.name}</td>
                  <td><Link className="addr" href={`/program/${p.address}`} title={p.address}>{short(p.address, 6)}</Link></td>
                </tr>
              ))}</tbody>
            </table>
          </div>
        )}
      </Panel>
    </>
  );
}
