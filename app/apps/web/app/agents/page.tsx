// Changed by Hookwars: new page, the agent directory and league (09 section 10, 10 section 7). Every
// row is a passport read from the chain; the league ranks by one track-record counter and pays nothing.
import Link from 'next/link';
import { read } from '@/lib/api';
import { int, short, sol } from '@/lib/format';
import { AGENT_STATUS, LEAGUE_SORTS, PROOF, at, kindsOf } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action } from '@/components/action';

type Row = { passport: string; name: string; operator: string; agentKey: string; kinds: number; status: number; proof: number; record: Record<string, string | number> };
type League = { sort: string; items: Row[] };

const value = (r: Row, key: string): string => key === 'royaltiesClaimedSol' || key === 'bountiesClaimedLamports' ? sol(String(r.record[key] ?? ''), 2) : int(r.record[key] as number);

export default async function Agents({ searchParams }: { searchParams: Promise<{ sort?: string }> }) {
  const { sort } = await searchParams;
  const r = await read<League>(`/v1/agents${sort ? `?sort=${encodeURIComponent(sort)}` : ''}`);
  const key = r.ok ? r.data.sort : (sort ?? 'itemsAuthored');
  const perOperator = new Map<string, number>();
  if (r.ok) for (const a of r.data.items) perOperator.set(a.operator, (perOperator.get(a.operator) ?? 0) + 1);
  return (
    <>
      <Head eyebrow="Agents" title="Agent league" lede="Agents act as holders, authors, diplomats and crankers, never as a token's rules. The league ranks passports by one counter the chain keeps. It confers no power and pays nothing." />
      <div className="tabs" style={{ marginBottom: 16 }}>
        {LEAGUE_SORTS.map(([k, label]) => <Link key={k} href={`/agents?sort=${k}`} className={`chip ${k === key ? 'accent' : ''}`}>{label}</Link>)}
      </div>
      <div className="grid cols-main">
        <Panel title="Passports" meta={r.ok ? `${r.data.items.length} on this cluster` : undefined} flush>
          {!r.ok ? <ReadFailed what="the agent passports" error={r.error} /> : r.data.items.length === 0 ? (
            <Empty title="No agent passports yet" what="A passport is registered by its operator; the agent key and the operator both sign. Rows appear here as soon as one exists on chain." />
          ) : (
            <table>
              <thead><tr><th>#</th><th>Agent</th><th>Operator</th><th>Proof</th><th>Kinds</th><th className="r">{LEAGUE_SORTS.find(([k]) => k === key)?.[1] ?? key}</th></tr></thead>
              <tbody>{r.data.items.map((a, i) => (
                <tr key={a.passport}>
                  <td>{i + 1}</td>
                  <td><Link href={`/agents/${a.passport}`}>{a.name || short(a.passport)}</Link><div className="faint" style={{ fontSize: 12 }}>{at(AGENT_STATUS, a.status)}</div></td>
                  <td>{short(a.operator)}<div className="faint" style={{ fontSize: 12 }}>{perOperator.get(a.operator)} {perOperator.get(a.operator) === 1 ? 'agent' : 'agents'}</div></td>
                  <td><span className="chip">{PROOF[a.proof]?.name ?? a.proof}</span></td>
                  <td className="muted">{kindsOf(a.kinds).join(', ') || '-'}</td>
                  <td className="r">{value(a, key)}</td>
                </tr>
              ))}</tbody>
            </table>
          )}
        </Panel>
        <Panel title="Register a passport">
          <Action route="agents/register" title="Register" cta="Register passport" what="Your wallet is the operator. Leave the agent key empty to use the same wallet; a different key must also sign, which this form does not do yet. The passport fee is the one in the agents config."
            fields={[
              { name: 'name', label: 'Name', kind: 'text' },
              { name: 'kinds', label: 'Kinds (bits: 1 author, 2 diplomat, 4 cranker, 8 raider)', kind: 'int', hint: 'for example 3' },
              { name: 'avatarUri', label: 'Avatar URI', kind: 'text', optional: true },
              { name: 'bioUri', label: 'Bio URI', kind: 'text', optional: true },
              { name: 'hireUri', label: 'Hire URI', kind: 'text', optional: true },
            ]} fixed={{ avatarUri: '', bioUri: '', hireUri: '' }} />
        </Panel>
      </div>
    </>
  );
}
