import type { BattleEvent, Page } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';

export default async function Generals() {
  const f = await read<Page<BattleEvent>>('/v1/feed?kind=raid');
  const by = new Map<string, number>();
  if (f.ok) for (const e of f.data.items) if (e.actor) by.set(e.actor, (by.get(e.actor) ?? 0) + Number(e.amount ?? 0));
  const top = [...by.entries()].sort((a, b) => b[1] - a[1]);
  return (
    <>
      <Head eyebrow="Generals" title="Top raiders across tokens" lede="Ranked by raid volume from indexed raids. Generals call raids; the title gives no control." />
      <Panel title="All tokens" flush>
        {!f.ok ? <ReadFailed what="raids" error={f.error} /> : top.length === 0 ? <Empty title="No raids yet" what="A raid is a buy of a token paid for by selling a rival its Raid item targets, in one swap_route." /> : (
          <table><thead><tr><th>Rank</th><th>Raider</th><th className="num">Volume (lamports)</th></tr></thead>
            <tbody>{top.map(([a, v], i) => <tr key={a}><td>{i + 1}</td><td className="addr">{short(a, 8)}</td><td className="num">{v.toLocaleString('en-US')}</td></tr>)}</tbody></table>
        )}
      </Panel>
    </>
  );
}
