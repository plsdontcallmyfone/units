// Changed by Hookwars: the item marketplace (10 section 1): live listings read from the chain,
// collections, and the list form. An item is a supply-1 token, so selling it sells its royalty stream.
import Link from 'next/link';
import { TEMPLATES } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action } from '@/components/action';

type Listing = { listing: string; seller: string; item: string; itemMint: string; priceLamports: string; createdAt: string; expiresAt: string; template: { template_id: number; level: number } | null };
type Collection = { collection: string; id: number; name: string; curator: string; templateIds: number[] };
const tname = (id: number | undefined) => TEMPLATES.find((t) => t.id === id)?.name ?? (id === undefined ? 'Item' : `template ${id}`);

export default async function Marketplace() {
  const [listings, cols] = await Promise.all([read<{ items: Listing[] }>('/v1/market/listings'), read<{ items: Collection[] }>('/v1/market/collections')]);
  const now = Date.now() / 1000;
  return (
    <>
      <Head eyebrow="Marketplace" title="Items for sale" lede="An item is a supply-1 token, so selling it sells its royalty stream. Before you sell, settle and claim: the buyer receives whatever is unclaimed."
        right={<Link className="btn sm" href="/marketplace/rentals">Rentals</Link>} />
      <div className="grid cols-main">
        <Panel title="Listings" meta={listings.ok ? `${listings.data.items.length} listed` : undefined} flush>
          {!listings.ok ? <ReadFailed what="the listings" error={listings.error} /> : listings.data.items.length === 0 ? (
            <Empty title="Nothing listed on this cluster" what="A holder lists an item at a price in SOL; the market keeps it in escrow until it sells or is delisted." />
          ) : (
            <table><thead><tr><th>Item</th><th>Seller</th><th className="r">Price</th><th>Expires</th></tr></thead>
              <tbody>{listings.data.items.map((l) => {
                const expired = Number(l.expiresAt) !== 0 && Number(l.expiresAt) < now;
                return (
                  <tr key={l.listing}>
                    <td><Link href={`/marketplace/items/${l.itemMint}`}>{tname(l.template?.template_id)}{l.template ? ` L${l.template.level}` : ''}</Link><div className="faint">{short(l.itemMint)}</div></td>
                    <td>{short(l.seller)}</td>
                    <td className="r">{sol(l.priceLamports, 4)}</td>
                    <td className={expired ? 'bad' : 'faint'}>{Number(l.expiresAt) === 0 ? 'no expiry' : expired ? 'expired' : when(l.expiresAt)}</td>
                  </tr>
                );
              })}</tbody></table>
          )}
        </Panel>
        <Panel title="List an item">
          <Action route="market/list" title="List" cta="List item" what="The item moves into the market's escrow. Expiry is a unix time in seconds; 0 lists with no expiry."
            fields={[{ name: 'itemMint', label: 'Item mint', kind: 'key' }, { name: 'priceLamports', label: 'Price', kind: 'sol' }, { name: 'expiresAt', label: 'Expires at', kind: 'int', hint: '0 for none' }]} />
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <div className="grid cols-main">
        <Panel title="Collections" meta="curated sets of templates" flush>
          {!cols.ok ? <ReadFailed what="collections" error={cols.error} /> : cols.data.items.length === 0 ? <Empty title="No collections yet" what="A curator names a set of templates; the set confers nothing on chain, it groups items for browsing." /> : (
            <table><thead><tr><th>#</th><th>Name</th><th>Curator</th><th>Templates</th></tr></thead>
              <tbody>{cols.data.items.map((c) => <tr key={c.collection}><td>{c.id}</td><td>{c.name}</td><td>{short(c.curator)}</td><td className="muted">{c.templateIds.map((t) => tname(t)).join(', ')}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Create a collection">
          <Action route="market/collections" title="Create" cta="Create collection" fields={[{ name: 'name', label: 'Name', kind: 'text' }, { name: 'templateIds', label: 'Template ids', kind: 'ints', hint: 'for example 1, 2, 9' }]} />
        </Panel>
      </div>
    </>
  );
}
