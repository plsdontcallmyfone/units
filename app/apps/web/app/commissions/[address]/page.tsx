// Changed by Hookwars: new page, one commission with its submissions and the actions on it.
import Link from 'next/link';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { COMMISSION_STATE, at, when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';
import type { CommissionDetail } from '@/lib/types';

export default async function CommissionPage({ params }: { params: Promise<{ address: string }> }) {
  const { address } = await params;
  const r = await read<CommissionDetail | null>(`/v1/commissions/${address}`);
  if (!r.ok) return <><Head eyebrow="Commission" title={short(address, 6)} /><Panel title="Commission"><ReadFailed what="this commission" error={r.error} /></Panel></>;
  if (!r.data) return <><Head eyebrow="Commission" title={short(address, 6)} /><Panel title="Commission"><Empty title="No commission at this address" what="It may have been paid or refunded and closed." /></Panel></>;
  const c = r.data;
  const brief = /^https:\/\//.test(c.briefUri) ? c.briefUri : null;
  const closed = Number(c.closesAt) < Date.now() / 1000;
  return (
    <>
      <Head eyebrow={`Commission · ${at(COMMISSION_STATE, c.state)}`} title={`Slot ${c.slot} of ${short(c.tokenMint, 6)}`} lede={brief ? <a href={brief} target="_blank" rel="noreferrer">Read the brief</a> : 'The brief link is not an https address, so it is not linked.'} />
      <div className="grid cols-main">
        <Panel title="Submissions" meta={`${c.submissions.length}`} flush>
          {c.submissions.length === 0 ? <Empty title="No submissions yet" what="An author submits an item they hold; the slot's own rule decides what is equipped." /> : (
            <table><thead><tr><th>Item</th><th>Author</th><th>Submitted</th><th /></tr></thead>
              <tbody>{c.submissions.map((s) => <tr key={s.submission}><td><Link href={`/armory/items/${s.item}`}>{short(s.item)}</Link></td><td>{short(s.submitter)}</td><td className="faint">{when(s.submittedAt)}</td><td>{c.winner === s.item ? <span className="chip ok">paid</span> : null}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Terms">
          <dl className="kv">
            <dt>Token</dt><dd><Link href={`/t/${c.tokenMint}`}>{short(c.tokenMint, 6)}</Link></dd>
            <dt>Bounty</dt><dd>{sol(c.bountyLamports, 4)}</dd>
            <dt>Vault</dt><dd>{sol(c.vaultLamports, 4)}</dd>
            <dt>Creator</dt><dd>{short(c.creator, 6)}</dd>
            <dt>Window</dt><dd>{when(c.opensAt)} to {when(c.closesAt)}</dd>
          </dl>
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      {c.state === 0 ? (
        <Panel title="Actions">
          <Actions>
            {!closed ? <Action route="commissions/submit" title="Submit an item" fixed={{ commission: address }} fields={[{ name: 'itemMint', label: 'Item mint', kind: 'key' }]} /> : null}
            <Action route="commissions/pay" title="Pay the winner" what="Anyone may crank this once the slot has equipped a submitted item." fixed={{ commission: address }} fields={[{ name: 'item', label: 'Equipped item', kind: 'key' }]} />
            {closed ? <Action route="commissions/refund" title="Refund the bounty" what="After the window, with no winner, the bounty returns to the creator." fixed={{ commission: address }} /> : null}
          </Actions>
        </Panel>
      ) : null}
    </>
  );
}
