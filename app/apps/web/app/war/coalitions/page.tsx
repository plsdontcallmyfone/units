// Changed by Hookwars: new page (app pass 5). Coalitions and boss pools (pass 4b, secfix3 M-6 and
// L-3): war tokens that equip a Coalition item naming the same id pool a shared chest, siege a
// rival together and get the chest back pro rata after the term; a season's boss pool pays the
// tokens that raided its boss. Every figure is a chain account's field.
import Link from 'next/link';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Captured = { mint: string; amount: string };
type Coalition = { address: string; id: number; count: number; members: string[]; contributed: string[]; createdAt: string; endsAt: string; fundedTotal: string; spentSiege: string; razedProceeds: string; paidCranks: string; returned: string; captured: Captured[]; dissolved: boolean };
type BossPool = { address: string; season: number; bossMint: string; funded: string; paid: string; sealed: boolean; totalVolume: string; toShare: string; effectiveAt: string };
type Data = { coalitions: Coalition[]; bossPools: BossPool[] };

const now = () => Math.floor(Date.now() / 1000);
function state(c: Coalition): [string, string] {
  if (c.dissolved) return ['dissolved', ''];
  if (Number(c.endsAt) > now()) return [c.contributed.every((x) => x === '0') ? 'open to join' : 'running', 'ok'];
  return ['term over', 'warn'];
}

export default async function Coalitions() {
  const r = await read<Data>('/v1/war/coalitions');
  const d = r.ok ? r.data : null;
  return (
    <>
      <Head eyebrow="War" title="Coalitions and boss pools" lede="A coalition is a shared war chest for tokens that all equipped a Coalition item naming it. Any such token joins before the first contribution; after the term the chest returns to the members by what they put in." right={<Link className="btn sm" href="/war">War room</Link>} />
      {!r.ok ? <Panel title="Coalitions"><ReadFailed what="coalitions" error={r.error} /></Panel> : (
        <>
          <Panel title="Coalitions" meta={`${d!.coalitions.length}`} flush>
            {d!.coalitions.length === 0 ? <Empty title="No coalition formed yet" what="Equip a Coalition item naming an id on two or more war tokens, then form it below. Each member consents by equipping." /> : (
              <table><thead><tr><th>Id</th><th>Members</th><th className="r">Funded</th><th className="r">Spent on sieges</th><th className="r">Raze proceeds</th><th className="r">Returned</th><th>Ends</th><th>State</th></tr></thead>
                <tbody>{d!.coalitions.map((c) => { const [label, tone] = state(c); return (
                  <tr key={c.address}><td>#{c.id}</td><td><div className="members">{c.members.map((m) => <Link key={m} href={`/t/${m}`}>{short(m)}</Link>)}</div></td>
                    <td className="r">{sol(c.fundedTotal)}</td><td className="r">{sol(c.spentSiege)}</td><td className="r">{sol(c.razedProceeds)}</td><td className="r">{sol(c.returned)}</td>
                    <td className="faint">{when(c.endsAt)}</td><td><span className={`chip ${tone}`}>{label}</span>{c.captured.some((x) => x.amount !== '0') ? <span className="chip warn">holds captured</span> : null}</td></tr>
                ); })}</tbody></table>
            )}
          </Panel>
          <div style={{ height: 16 }} />
          <Panel title="Boss pools" meta="by season" flush>
            {d!.bossPools.length === 0 ? <Empty title="No boss pool opened" what="The war admin names a season's boss token; a share of protocol fees funds its pool once the timelock passes, and it pays the tokens that raided the boss." /> : (
              <table><thead><tr><th>Season</th><th>Boss</th><th className="r">Funded</th><th className="r">Paid out</th><th className="r">Raid volume</th><th>State</th></tr></thead>
                <tbody>{d!.bossPools.map((b) => <tr key={b.address}><td>{b.season}</td><td><Link href={`/t/${b.bossMint}`}>{short(b.bossMint)}</Link></td><td className="r">{sol(b.funded)}</td><td className="r">{sol(b.paid)}</td><td className="r">{b.sealed ? sol(b.totalVolume) : '-'}</td>
                  <td>{b.sealed ? <span className="chip">sealed</span> : Number(b.effectiveAt) > now() ? <span className="chip warn">funds from {when(b.effectiveAt)}</span> : <span className="chip ok">funding</span>}</td></tr>)}</tbody></table>
            )}
          </Panel>
        </>
      )}
      <div style={{ height: 16 }} />
      <Panel title="Act">
        <Actions>
          <Action route="war/coalition/form" title="Form a coalition" what="Every member must already equip a Coalition item naming this id, and be at war." fields={[{ name: 'id', label: 'Coalition id', kind: 'int' }, { name: 'termSecs', label: 'Term (seconds)', kind: 'amount' }, { name: 'members', label: 'Member token mints', kind: 'keys' }]} />
          <Action route="war/coalition/join" title="Join a coalition" what="While the term runs and before any member has contributed." fields={[{ name: 'id', label: 'Coalition id', kind: 'int' }, { name: 'mint', label: 'Your war token', kind: 'key' }]} />
        </Actions>
      </Panel>
    </>
  );
}
