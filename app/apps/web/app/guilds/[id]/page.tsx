// Changed by Hookwars: new page, one guild hall: treasury, officers, actions and their state.
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Kind = { name: 'SpendSol'; to: string; lamports: string } | { name: 'SpendToken'; mint: string; to: string; amount: string } | { name: 'SetOfficers'; officers: string[]; threshold: number };
type GuildAction = { action: string; nonce: string; kind: Kind; approvals: number; proposedAt: string; eta: string; executed: boolean };
type Guild = { guild: string; id: number; name: string; officers: string[]; threshold: number; treasury: string; treasuryLamports: string | null; createdAt: string; actions: GuildAction[] };

function describe(k: Kind): string {
  if (k.name === 'SpendSol') return `Send ${sol(k.lamports, 4)} to ${short(k.to)}`;
  if (k.name === 'SpendToken') return `Send ${k.amount} base units of ${short(k.mint)} to ${short(k.to)}`;
  return `Set ${k.officers.length} officers, threshold ${k.threshold}`;
}

export default async function GuildPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const n = Number(id);
  if (!Number.isInteger(n) || n < 0) return <><Head eyebrow="Guild" title={id} /><Panel title="Guild"><Empty title="Not a guild id" what="Guild ids are whole numbers." /></Panel></>;
  const r = await read<Guild | null>(`/v1/guilds/${n}`);
  if (!r.ok) return <><Head eyebrow="Guild" title={`Guild ${n}`} /><Panel title="Guild"><ReadFailed what="this guild" error={r.error} /></Panel></>;
  if (!r.data) return <><Head eyebrow="Guild" title={`Guild ${n}`} /><Panel title="Guild"><Empty title="No guild with this id" what="Guild ids count up from 0 as guilds are founded." /></Panel></>;
  const g = r.data;
  const now = Date.now() / 1000;
  return (
    <>
      <Head eyebrow={`Guild ${g.id}`} title={g.name} lede={`${g.threshold} of ${g.officers.length} officers approve each action. Founded ${when(g.createdAt)}.`} />
      <div className="grid cols-main">
        <Panel title="Actions" meta={`${g.actions.length}`} flush>
          {g.actions.length === 0 ? <Empty title="No actions proposed" what="An officer proposes a spend or an officer change; it executes after the threshold and the timelock." /> : (
            <table><thead><tr><th>#</th><th>Action</th><th className="r">Approvals</th><th>State</th></tr></thead>
              <tbody>{g.actions.map((a) => <tr key={a.action}><td>{a.nonce}</td><td>{describe(a.kind)}</td><td className="r">{a.approvals} of {g.threshold}</td><td>{a.executed ? <span className="chip ok">executed</span> : Number(a.eta) > now ? <span className="chip">timelock until {when(a.eta)}</span> : <span className="chip warn">ready when approved</span>}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Treasury">
          <dl className="kv"><dt>Balance</dt><dd>{sol(g.treasuryLamports, 4)}</dd><dt>Address</dt><dd>{short(g.treasury, 6)}</dd><dt>Officers</dt><dd>{g.officers.map((o) => short(o)).join(', ')}</dd></dl>
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Act">
        <Actions>
          <Action route="guilds/deposit" title="Deposit SOL" fixed={{ guildId: g.id }} fields={[{ name: 'lamports', label: 'Amount', kind: 'sol' }]} />
          <Action route="guilds/propose" title="Propose a SOL spend" what="Officers only." fixed={{ guildId: g.id, kind: { name: 'SpendSol' } }} fields={[{ name: 'kind.to', label: 'To', kind: 'key' }, { name: 'kind.lamports', label: 'Amount', kind: 'sol' }]} />
          <Action route="guilds/propose" title="Propose new officers" what="Officers only." fixed={{ guildId: g.id, kind: { name: 'SetOfficers' } }} fields={[{ name: 'kind.officers', label: 'Officers', kind: 'keys', hint: 'comma separated' }, { name: 'kind.threshold', label: 'Threshold', kind: 'int' }]} />
          {g.actions.filter((a) => !a.executed).map((a) => (
            <Action key={a.action} route={a.approvals >= g.threshold && Number(a.eta) <= now ? 'guilds/execute' : 'guilds/approve'} title={a.approvals >= g.threshold && Number(a.eta) <= now ? `Execute action ${a.nonce}` : `Approve action ${a.nonce}`} what={describe(a.kind)} fixed={{ guildId: g.id, nonce: a.nonce }} />
          ))}
        </Actions>
      </Panel>
    </>
  );
}
