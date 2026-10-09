// Changed by Hookwars: new page, item rental offers and active leases (10 section 4), read from the chain.
import Link from 'next/link';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { LEASE_STATE, at, when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';

type Lease = { lease: string; lessor: string; item: string; itemMint: string; tokenMint: string; slot: number; rentBps: number; feeLamports: string; termSecs: number; startsAt: string; endsAt: string; state: number };

export default async function Rentals() {
  const r = await read<{ items: Lease[] }>('/v1/market/leases');
  return (
    <>
      <Head eyebrow="Marketplace" title="Rentals" lede="An owner offers an item to one token's slot for a fixed term, for a one-time fee and a share of what the item collects there. The token's community still equips it by the slot rule." />
      <Panel title="Offers and leases" meta={r.ok ? `${r.data.items.length}` : undefined} flush>
        {!r.ok ? <ReadFailed what="leases" error={r.error} /> : r.data.items.length === 0 ? <Empty title="No rental offers" what="Offer one from an item's page; it shows here until it is accepted, withdrawn or ends." /> : (
          <table><thead><tr><th>Item</th><th>Token, slot</th><th>State</th><th className="r">Fee</th><th className="r">Rent</th><th>Term</th></tr></thead>
            <tbody>{r.data.items.map((l) => (
              <tr key={l.lease}>
                <td><Link href={`/marketplace/items/${l.itemMint}`}>{short(l.itemMint)}</Link><div className="faint">by {short(l.lessor)}</div></td>
                <td><Link href={`/t/${l.tokenMint}`}>{short(l.tokenMint)}</Link>, slot {l.slot}</td>
                <td><span className={`chip ${l.state === 1 ? 'ok' : ''}`}>{at(LEASE_STATE, l.state)}</span></td>
                <td className="r">{sol(l.feeLamports, 4)}</td>
                <td className="r">{(l.rentBps / 100).toLocaleString('en-US')}%</td>
                <td className="faint">{l.state === 1 ? `until ${when(l.endsAt)}` : `${Math.round(l.termSecs / 3600)} h`}</td>
              </tr>
            ))}</tbody></table>
        )}
      </Panel>
    </>
  );
}
