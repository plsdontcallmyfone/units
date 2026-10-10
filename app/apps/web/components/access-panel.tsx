// Changed by Hookwars: new file (app pass 5). An item's access (pass 4a E-1, spec 14): who may
// equip it and on what terms, read from the chain (the item's mode, its AccessPolicy and every
// Approval naming it), with the holder's forms. Trades are never gated; only equips are.
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Terms = { priceLamports: string; termSecs: number; per: number; maxLive: number };
type Approval = { address: string; tokenMint: string; approvedBy: string; approvedAt: string; revokeAfter: string };
type Access = { mode: number; exclusive: boolean; policy: { licenceTerms: Terms | null; holderAtSet: string; updatedAt: string } | null; approvals: Approval[] } | null;

export const MODES: [string, string, string][] = [
  ['0', 'Open', 'Any token may equip it.'],
  ['1', 'Gated', 'Only tokens the holder approved.'],
  ['2', 'Licensed', 'Tokens that hold a live licence, bought on the terms below.'],
  ['3', 'Leased', 'Only the token that leases it.'],
  ['4', 'Exclusive', 'One token at a time.'],
];

const term = (s: number) => (s % 86_400 === 0 ? `${s / 86_400} d` : s % 3_600 === 0 ? `${s / 3_600} h` : `${s} s`);

export async function AccessPanel({ itemMint }: { itemMint: string }) {
  const r = await read<Access>(`/v1/access/${itemMint}`);
  if (!r.ok) return <Panel title="Access"><ReadFailed what="this item's access" error={r.error} /></Panel>;
  const a = r.data;
  if (!a) return <Panel title="Access"><Empty title="No access record" what="The chain has no armory item at this mint." /></Panel>;
  const mode = MODES[a.mode] ?? [String(a.mode), `Mode ${a.mode}`, ''];
  const t = a.policy?.licenceTerms ?? null;
  return (
    <Panel title="Access" meta={a.policy ? `set ${when(a.policy.updatedAt)}` : 'never set: open by default'}>
      <div className="access-head">
        <div>
          <div className="eyebrow">Who may equip it</div>
          <div className="access-mode">{mode[1]}{a.exclusive ? <span className="chip warn">exclusive</span> : null}</div>
          <p className="muted">{mode[2]} Buying and selling the item is never gated.</p>
        </div>
        {t ? (
          <dl className="kv access-terms">
            <dt>Licence price</dt><dd className="num">{sol(t.priceLamports, 6)}</dd>
            <dt>Term</dt><dd className="num">{term(t.termSecs)}</dd>
            <dt>Live at once</dt><dd className="num">{t.maxLive}</dd>
          </dl>
        ) : null}
      </div>
      {a.mode === 1 || a.approvals.length ? (
        a.approvals.length === 0 ? <Empty title="No token approved yet" what="While Gated, only the tokens the holder approves can equip this item. Approve one below." /> : (
          <table><thead><tr><th>Approved token</th><th>By</th><th>Since</th><th>Equips stop</th></tr></thead>
            <tbody>{a.approvals.map((x) => <tr key={x.address}><td className="addr">{short(x.tokenMint, 6)}</td><td>{short(x.approvedBy)}</td><td className="faint">{when(x.approvedAt)}</td><td className="faint">{x.revokeAfter === '0' ? 'not revoked' : when(x.revokeAfter)}</td></tr>)}</tbody></table>
        )
      ) : null}
      <Actions>
        <Action route="access/set" title="Set access" what="Only the holder sets it. A Licensed mode needs terms; a licence priced above the first tier needs the Builder level." fixed={{ itemMint }} fields={[
          { name: 'mode', label: 'Mode', kind: 'int', choices: MODES.map(([v, l]) => [v, l]) },
          { name: 'exclusive', label: 'Exclusive', kind: 'bool' },
          { name: 'licenceTerms', label: 'Licence terms (Licensed only)', kind: 'json', optional: true, hint: '{"priceLamports":"100000000","termSecs":86400,"per":0,"maxLive":3}' },
        ]} />
        <Action route="access/approve" title="Approve a token" what="A Gated item may then be equipped by this token." fixed={{ itemMint }} fields={[{ name: 'tokenMint', label: 'Token mint', kind: 'key' }]} />
        <Action route="access/revoke" title="Revoke an approval" what="The token keeps the item until the notice ends, then anyone can enforce the revert." fixed={{ itemMint }} fields={[{ name: 'tokenMint', label: 'Token mint', kind: 'key' }]} />
        {a.mode === 2 ? <Action route="licences/buy" title="Buy a licence" what={t ? `Your token may equip this item for ${term(t.termSecs)} at ${sol(t.priceLamports, 6)}.` : 'On the terms the holder set.'} fixed={{ itemMint }} fields={[{ name: 'tokenMint', label: 'Your token mint', kind: 'key' }, { name: 'renew', label: 'Renew a live licence', kind: 'bool' }]} /> : null}
      </Actions>
    </Panel>
  );
}
