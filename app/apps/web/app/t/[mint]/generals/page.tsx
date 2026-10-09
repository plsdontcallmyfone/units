import type { General } from '@hookwars/shared';
import { read } from '@/lib/api';
import { int, short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';

export default async function TokenGenerals({ params }: { params: Promise<{ mint: string }> }) {
  const { mint } = await params;
  const g = await read<General[]>(`/v1/launches/${mint}/generals`);
  return (
    <>
      <Head eyebrow={`Generals · ${short(mint, 6)}`} title="Top raiders" lede="Generals are the top raiders by on-chain points. It is a title; it gives no control." />
      <Panel title="This season" flush>
        {!g.ok ? <ReadFailed what="generals" error={g.error} /> : g.data.length === 0 ? <Empty title="No raid points this season" what="Points are stamped into a holding when it buys this token by selling a rival its Raid targets. Ranks read them from hook data." /> : (
          <table><thead><tr><th>Rank</th><th>Holder</th><th className="num">Raid points</th><th className="num">Raid volume</th></tr></thead>
            <tbody>{g.data.map((x) => <tr key={x.owner}><td>{x.rank}</td><td className="addr">{x.owner}</td><td className="num">{int(x.raidPoints)}</td><td className="num">{int(x.raidVolume)}</td></tr>)}</tbody></table>
        )}
      </Panel>
    </>
  );
}
