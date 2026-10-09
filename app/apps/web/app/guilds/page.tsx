// Changed by Hookwars: new page, guild halls (10 section 6): a shared treasury spent by officers
// under a threshold and a timelock. Read from the chain.
import Link from 'next/link';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action } from '@/components/action';

type Guild = { guild: string; id: number; name: string; officers: string[]; threshold: number; actions: string; createdAt: string; treasuryLamports: string | null };

export default async function Guilds() {
  const r = await read<{ items: Guild[] }>('/v1/guilds');
  return (
    <>
      <Head eyebrow="Guilds" title="Guild halls" lede="A guild is a treasury anyone can deposit into and its officers spend: an action needs the threshold of officer approvals, then waits out the timelock before anyone executes it." />
      <div className="grid cols-main">
        <Panel title="Guilds" meta={r.ok ? `${r.data.items.length}` : undefined} flush>
          {!r.ok ? <ReadFailed what="guilds" error={r.error} /> : r.data.items.length === 0 ? <Empty title="No guilds yet" what="Found one with the form; the founder is its first officer." /> : (
            <table><thead><tr><th>#</th><th>Guild</th><th>Officers</th><th className="r">Treasury</th><th>Founded</th></tr></thead>
              <tbody>{r.data.items.map((g) => <tr key={g.guild}><td>{g.id}</td><td><Link href={`/guilds/${g.id}`}>{g.name}</Link></td><td>{g.threshold} of {g.officers.length}</td><td className="r">{sol(g.treasuryLamports, 3)}</td><td className="faint">{when(g.createdAt)}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Found a guild">
          <Action route="guilds/create" title="Found" cta="Found guild" fields={[{ name: 'name', label: 'Name', kind: 'text' }]} />
        </Panel>
      </div>
    </>
  );
}
