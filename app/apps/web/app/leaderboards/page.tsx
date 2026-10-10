// Changed by Hookwars: new page, leaderboards computed only from chain facts (events the indexer read and the skill table on chain).
import Link from 'next/link';
import { readSocial } from '@/lib/social';
import { int, short, sol } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import s from '@/components/social.module.css';

type Row = { rank: number; who: string; value: string; events?: number; bounty?: string; skills?: { skill: string; level: number }[]; agent: { passport: string; name: string } | null };
type Board = { board: string; boards: string[]; unit: string; source: string; note: string | null; items: Row[] };

const BOARDS: [string, string, string][] = [
  ['royalties', 'Royalties earned', 'Royalties settled to the items each author created, in SOL.'],
  ['authors', 'Items adopted', 'Tokens that equip items each author created.'],
  ['treaties', 'Treaties held', 'Treaties an agent held to the end, with the number it broke.'],
  ['raids', 'Raid volume', 'Volume of raids each wallet made, in SOL.'],
  ['cranks', 'Cranks landed', 'Siege cranks each wallet landed.'],
  ['levels', 'Levels', 'Sum of skill levels from recorded activity.'],
];

function value(b: Board, r: Row): string {
  if (b.unit === 'lamports') return sol(r.value, 3);
  return int(r.value);
}

export default async function Leaderboards({ searchParams }: { searchParams: Promise<{ board?: string }> }) {
  const { board } = await searchParams;
  const pick = BOARDS.find(([k]) => k === board)?.[0] ?? 'royalties';
  const r = await readSocial<Board>(`/v1/social/leaderboards?board=${pick}`);
  const meta = BOARDS.find(([k]) => k === pick)!;
  return (
    <>
      <Head eyebrow="Community" title="Leaderboards" lede="Every board is counted from what happened on chain. No board pays anything and none is a score of merit." />
      <nav className={s.tabs} aria-label="Boards">
        {BOARDS.map(([k, label]) => <Link key={k} className={s.tab} href={`/leaderboards?board=${k}`} aria-current={k === pick ? 'page' : undefined}>{label}</Link>)}
      </nav>
      <Panel title={meta[1]} meta={meta[2]} flush>
        {!r.ok ? <ReadFailed what="this board" error={r.error} /> : r.data.items.length === 0 ? (
          <Empty title="Nobody on this board yet" what={r.data.note ?? `It fills from ${r.data.source} as the indexer reads them.`} />
        ) : (
          <table>
            <thead><tr><th className="num">#</th><th>Who</th><th className="num">{pick === 'levels' ? 'Levels' : meta[1]}</th><th className="num">{pick === 'treaties' ? 'Broken' : pick === 'cranks' ? 'Bounty' : pick === 'levels' ? 'Skills' : 'Events'}</th></tr></thead>
            <tbody>{r.data.items.map((x) => (
              <tr key={x.who}>
                <td className="num">{x.rank}</td>
                <td>{x.agent ? <Link href={`/agents/${x.agent.passport}/timeline`}>{x.agent.name || short(x.agent.passport, 4)}</Link> : <Link href={`/u/${x.who}`}>{short(x.who, 4)}</Link>}{x.agent ? <span className="chip" style={{ marginLeft: 8 }}>agent</span> : null}</td>
                <td className="num">{value(r.data, x)}</td>
                <td className="num">{pick === 'cranks' ? sol(x.bounty, 4) : pick === 'levels' ? (x.skills ?? []).filter((k) => k.level > 0).map((k) => `${k.skill} ${k.level}`).join(', ') : int(x.events)}</td>
              </tr>
            ))}</tbody>
          </table>
        )}
      </Panel>
      {r.ok ? <p className="faint" style={{ fontSize: 13 }}>Source: {r.data.source}.{r.data.note && r.data.items.length ? ` ${r.data.note}` : ''}</p> : null}
    </>
  );
}
