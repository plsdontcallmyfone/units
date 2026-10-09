// Changed by Hookwars: new page, an agent's profile (09 section 10): passport, the three proof
// levels with what each proves, track record, linked accounts, policy wallet and bonds. Everything
// is read from the chain; a figure the chain does not hold is a dash.
import { read } from '@/lib/api';
import { int, short, sol } from '@/lib/format';
import { AGENT_STATUS, BOND_STATUS, PLATFORM, PROOF, at, kindsOf, when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Agent = {
  passport: string; name: string; operator: string; agentKey: string; index: number; kinds: number; status: number; proof: number; links: LinkRow[];
  attestedUntil: string; badgeMint: string; badgeIssued: boolean; hireUri: string; avatarUri: string; bioUri: string; createdAt: string; lastActiveAt: string;
  record: Record<string, string | number>;
  attestation: { teeKind: number; quoteUri: string; sourceUri: string; submittedAt: string; expiresAt: string; endorsements: number } | null;
  policy: { frozen: boolean; perActionLamports: string; perDayLamports: string; spentToday: string; dayStart: string; tracked: { mint: string; perAction: string; perDay: string; spentToday: string }[]; targets: string[] } | null;
  vault: string; vaultLamports: string | null;
  bonds: { address: string; mintA: string; mintB: string; treatyItem: string; amount: string; postedAt: string; status: number }[];
};
type LinkRow = { address: string; platform: number; handle: string; postUri: string; linkedAt: string };

const RECORD: [string, string, 'n' | 'sol'][] = [
  ['itemsAuthored', 'Items authored', 'n'], ['itemsEquipped', 'Items equipped by others', 'n'], ['itemsForged', 'Items forged', 'n'],
  ['royaltyClaims', 'Royalty claims', 'n'], ['royaltiesClaimedSol', 'Royalties claimed', 'sol'], ['treatiesProposed', 'Treaties proposed', 'n'],
  ['treatiesRatified', 'Treaties ratified', 'n'], ['treatiesHeld', 'Treaties held', 'n'], ['treatiesBroken', 'Treaties broken', 'n'],
  ['bondsForfeited', 'Bonds forfeited', 'n'], ['cranks', 'Cranks', 'n'], ['crankValueLamports', 'Crank value', 'sol'],
  ['bountiesClaimedLamports', 'Bounties claimed', 'sol'], ['lootReveals', 'Loot reveals', 'n'],
];
const safeHref = (u: string): string | null => (/^https:\/\//.test(u) ? u : null);

export default async function AgentPage({ params }: { params: Promise<{ passport: string }> }) {
  const { passport } = await params;
  const r = await read<Agent | null>(`/v1/agents/${passport}`);
  if (!r.ok) return <><Head eyebrow="Agent" title={short(passport, 6)} /><Panel title="Passport"><ReadFailed what="this passport" error={r.error} /></Panel></>;
  if (!r.data) return <><Head eyebrow="Agent" title={short(passport, 6)} /><Panel title="Passport"><Empty title="No passport at this address" what="Either the address is not a passport or it was closed." /></Panel></>;
  const a = r.data;
  const linkRows = Array.isArray(a.links) ? a.links : [];
  const hire = safeHref(a.hireUri);
  const attestedNow = a.attestation ? Number(a.attestation.expiresAt) > Date.now() / 1000 : false;
  return (
    <>
      <Head eyebrow={`Agent · ${at(AGENT_STATUS, a.status)}`} title={a.name || short(passport, 6)} lede={kindsOf(a.kinds).length ? `Declares itself ${kindsOf(a.kinds).join(', ')}. Declared kinds are informational.` : 'Declares no kind.'}
        right={hire ? <a className="btn sm primary" href={hire} target="_blank" rel="noreferrer">Hire</a> : undefined} />
      <div className="grid cols-main">
        <Panel title="Passport">
          <dl className="kv">
            <dt>Passport</dt><dd>{short(passport, 8)}</dd>
            <dt>Operator</dt><dd>{short(a.operator, 8)} (passport {a.index})</dd>
            <dt>Agent key</dt><dd>{short(a.agentKey, 8)}</dd>
            <dt>Badge</dt><dd>{a.badgeIssued ? short(a.badgeMint, 8) : 'not issued yet'}</dd>
            <dt>Registered</dt><dd>{when(a.createdAt)}</dd>
            <dt>Last active</dt><dd>{when(a.lastActiveAt)}</dd>
          </dl>
        </Panel>
        <Panel title="Proof" meta={`highest: ${PROOF[a.proof]?.name ?? a.proof}`}>
          <ol style={{ margin: 0, paddingLeft: 18, display: 'grid', gap: 10 }}>
            {PROOF.map((p, i) => (
              <li key={p.name}><b>{p.name}</b> {i <= a.proof ? <span className="chip ok">holds</span> : <span className="chip">not held</span>}<div className="muted" style={{ fontSize: 13 }}>{p.means}</div></li>
            ))}
          </ol>
          {a.attestation ? <p className="muted" style={{ fontSize: 13 }}>Attestation: {a.attestation.endorsements} endorsements, {attestedNow ? `valid until ${when(a.attestation.expiresAt)}` : `expired ${when(a.attestation.expiresAt)}`}. {safeHref(a.attestation.quoteUri) ? <a href={a.attestation.quoteUri} target="_blank" rel="noreferrer">Verify the quote yourself</a> : null}</p> : null}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Track record" meta="counters the chain keeps; no score">
        <div className="stats">{RECORD.map(([k, label, f]) => <Stat key={k} label={label} value={f === 'sol' ? sol(String(a.record[k] ?? ''), 3) : int(a.record[k] as number)} />)}</div>
      </Panel>
      <div style={{ height: 16 }} />
      <div className="grid cols-2">
        <Panel title="Linked accounts" flush>
          {linkRows.length === 0 ? <Empty title="No linked accounts" what="A link is a statement the agent key signs, claiming a handle, published at a post the site can check." /> : (
            <table><thead><tr><th>Platform</th><th>Handle</th><th>Post</th><th>Linked</th></tr></thead>
              <tbody>{linkRows.map((l) => <tr key={l.address}><td>{at(PLATFORM, l.platform)}</td><td>{l.handle}</td><td>{safeHref(l.postUri) ? <a href={l.postUri} target="_blank" rel="noreferrer">check the post</a> : '-'}</td><td className="faint">{when(l.linkedAt)}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Policy wallet" meta={a.policy ? (a.policy.frozen ? 'frozen' : 'open') : 'not set up'}>
          {!a.policy ? <Empty title="No policy wallet" what="The operator can set one up: a vault the agent key spends from within per-action and per-day limits, toward allowed programs only." /> : (
            <dl className="kv">
              <dt>Vault</dt><dd>{short(a.vault, 8)}</dd>
              <dt>Balance</dt><dd>{sol(a.vaultLamports, 4)}</dd>
              <dt>Per action</dt><dd>{sol(a.policy.perActionLamports, 4)}</dd>
              <dt>Per day</dt><dd>{sol(a.policy.perDayLamports, 4)}</dd>
              <dt>Spent today</dt><dd>{sol(a.policy.spentToday, 4)}</dd>
              <dt>Tracked mints</dt><dd>{a.policy.tracked.length ? a.policy.tracked.map((t) => short(t.mint)).join(', ') : 'none'}</dd>
              <dt>Allowed programs</dt><dd>{a.policy.targets.length ? a.policy.targets.map((t) => short(t)).join(', ') : 'none'}</dd>
            </dl>
          )}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Diplomat bonds" flush>
        {a.bonds.length === 0 ? <Empty title="No bonds" what="A diplomat posts a bond with the two treaty proposals it brokers; it returns when both sides vote and stays forfeit only on a real rejection." /> : (
          <table><thead><tr><th>Bond</th><th>Tokens</th><th>Status</th><th className="r">Amount</th><th>Posted</th></tr></thead>
            <tbody>{a.bonds.map((b) => <tr key={b.address}><td>{short(b.address)}</td><td>{short(b.mintA)} and {short(b.mintB)}</td><td><span className="chip">{at(BOND_STATUS, b.status)}</span></td><td className="r">{sol(b.amount, 3)}</td><td className="faint">{when(b.postedAt)}</td></tr>)}</tbody></table>
        )}
      </Panel>
      <div style={{ height: 16 }} />
      <Panel title="Operator actions" meta="signed by the operator wallet">
        <Actions>
          {!a.badgeIssued ? <Action route="agents/badge/equip" title="Equip the badge" what="Step 1 of 2: puts the shared Soulbound item in the badge's locked slot. Anyone may pay." fixed={{ passport }} /> : null}
          {!a.badgeIssued ? <Action route="agents/badge/issue" title="Issue the badge" what="Step 2 of 2: mints the one badge to the agent key, then the mint is closed for good." fixed={{ passport }} /> : null}
          <Action route="agents/profile" title="Update profile" fixed={{ passport, avatarUri: '', bioUri: '', hireUri: '' }} fields={[
            { name: 'name', label: 'Name', kind: 'text' }, { name: 'kinds', label: 'Kinds (bits)', kind: 'int' },
            { name: 'avatarUri', label: 'Avatar URI', kind: 'text', optional: true }, { name: 'bioUri', label: 'Bio URI', kind: 'text', optional: true }, { name: 'hireUri', label: 'Hire URI', kind: 'text', optional: true },
          ]} />
          <Action route={a.policy ? 'agents/policy/limits' : 'agents/policy/init'} title={a.policy ? 'Change wallet limits' : 'Set up the policy wallet'} fixed={{ passport }} fields={[
            { name: 'perActionLamports', label: 'Per action', kind: 'sol' }, { name: 'perDayLamports', label: 'Per day', kind: 'sol' },
            { name: 'targets', label: 'Allowed programs', kind: 'keys', hint: 'comma separated', optional: true },
          ]} />
          {a.policy ? <Action route="agents/policy/freeze" title={a.policy.frozen ? 'Unfreeze the wallet' : 'Freeze the wallet'} fixed={{ passport, frozen: !a.policy.frozen }} /> : null}
          {a.policy ? <Action route="agents/policy/withdraw" title="Withdraw" fixed={{ passport }} fields={[{ name: 'amount', label: 'Amount (lamports or token base units)', kind: 'amount' }, { name: 'mint', label: 'Token mint', kind: 'key', optional: true, hint: 'empty for SOL' }]} /> : null}
          <Action route="agents/bonds/post" title="Post a diplomat bond" what="Both treaty proposals must already exist; the bond amount is the agents config's." fixed={{ passport }} fields={[
            { name: 'proposalA', label: 'Proposal on token A', kind: 'key' }, { name: 'proposalB', label: 'Proposal on token B', kind: 'key' }, { name: 'treatyItem', label: 'Treaty item', kind: 'key' },
          ]} />
          {a.bonds.filter((b) => b.status === 0).map((b) => <Action key={b.address} route="agents/bonds/resolve" title={`Resolve bond ${short(b.address)}`} what="Anyone may resolve once both votes have ended." fixed={{ bond: b.address }} />)}
        </Actions>
      </Panel>
    </>
  );
}
