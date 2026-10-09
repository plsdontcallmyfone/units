// Changed by Hookwars: new page, commissions (10 section 5): a community posts a bounty for a new
// hook for one slot; authors submit items; the winner the slot equips is paid. Read from the chain.
import Link from 'next/link';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { COMMISSION_STATE, at, when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action } from '@/components/action';
import type { CommissionRow } from '@/lib/types';


export default async function Commissions() {
  const r = await read<{ items: CommissionRow[] }>('/v1/commissions');
  return (
    <>
      <Head eyebrow="Commissions" title="Bounties for new hooks" lede="A creator escrows a bounty for one slot of a token. Authors submit items; when the slot equips a submitted item by its own rule, its author is paid. Unpaid bounties refund after the window." />
      <div className="grid cols-main">
        <Panel title="Commissions" meta={r.ok ? `${r.data.items.length}` : undefined} flush>
          {!r.ok ? <ReadFailed what="commissions" error={r.error} /> : r.data.items.length === 0 ? <Empty title="No commissions yet" what="Open one with the form; it appears here with its bounty, its window and every submission." /> : (
            <table><thead><tr><th>Token, slot</th><th>State</th><th className="r">Bounty</th><th className="r">Submissions</th><th>Closes</th></tr></thead>
              <tbody>{r.data.items.map((c) => (
                <tr key={c.commission}>
                  <td><Link href={`/commissions/${c.commission}`}>{short(c.tokenMint)}, slot {c.slot}</Link><div className="faint">by {short(c.creator)}</div></td>
                  <td><span className={`chip ${c.state === 0 ? 'ok' : ''}`}>{at(COMMISSION_STATE, c.state)}</span></td>
                  <td className="r">{sol(c.bountyLamports, 3)}</td>
                  <td className="r">{c.submissions}</td>
                  <td className="faint">{when(c.closesAt)}</td>
                </tr>
              ))}</tbody></table>
          )}
        </Panel>
        <Panel title="Open a commission">
          <Action route="commissions/open" title="Open" cta="Open commission" what="The bounty moves into the commission's vault. The nonce is any number not used before for this token."
            fields={[
              { name: 'tokenMint', label: 'Token', kind: 'key' }, { name: 'slot', label: 'Slot', kind: 'int' }, { name: 'nonce', label: 'Nonce', kind: 'amount' },
              { name: 'briefUri', label: 'Brief URI', kind: 'text', hint: 'https://...' }, { name: 'bountyLamports', label: 'Bounty', kind: 'sol' }, { name: 'windowSecs', label: 'Window (seconds)', kind: 'int' },
            ]} />
        </Panel>
      </div>
    </>
  );
}
