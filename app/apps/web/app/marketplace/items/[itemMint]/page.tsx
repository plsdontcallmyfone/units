// Changed by Hookwars: new page, one item on the market (10 sections 1, 2, 4): its listing, the
// price history drawn from sale events only, its lineage from forge events, rental state, and the
// actions its owner, a buyer or a lessee can take.
import Link from 'next/link';
import { TEMPLATES, itemSentence } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { LEASE_STATE, at, when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Node = { item: string; level: number | null; parents: Node[] };
type Sale = { signature: string; slot: string; ts: string | number; seller: string; buyer: string; price: string; fee: string; resale: string };
type MarketItem = {
  item: string; itemMint: string;
  data: { templateId: number; params: number[]; level: number; author: string; royaltyBps: number; source: number; equippedCount: number };
  listing: { seller: string; priceLamports: string; expiresAt: string } | null;
  lease: { lessor: string; tokenMint: string; slot: number; rentBps: number; feeLamports: string; termSecs: number; endsAt: string; state: number } | null;
  sales: Sale[]; lineage: Node & { children: string[] };
};

function Tree({ node }: { node: Node }) {
  return (
    <ul className="tree">
      <li><Link href={`/armory/items/${node.item}`}>{short(node.item)}</Link>{node.level !== null ? <span className="faint"> level {node.level}</span> : null}
        {node.parents.length ? <Tree2 nodes={node.parents} /> : null}
      </li>
    </ul>
  );
}
function Tree2({ nodes }: { nodes: Node[] }) {
  return <ul className="tree">{nodes.map((n) => <li key={n.item}><Link href={`/armory/items/${n.item}`}>{short(n.item)}</Link>{n.level !== null ? <span className="faint"> level {n.level}</span> : <span className="faint"> minted or looted</span>}{n.parents.length ? <Tree2 nodes={n.parents} /> : null}</li>)}</ul>;
}

/** Sale prices in order as a line; one point per sale, nothing between them. */
function PriceLine({ sales }: { sales: Sale[] }) {
  const ps = sales.map((s) => Number(s.price));
  const max = Math.max(...ps), min = Math.min(...ps);
  const x = (i: number) => (ps.length === 1 ? 150 : 8 + (i * 284) / (ps.length - 1));
  const y = (p: number) => (max === min ? 60 : 110 - ((p - min) / (max - min)) * 100);
  return (
    <svg className="spark" viewBox="0 0 300 120" role="img" aria-label={`${ps.length} sales, from ${sol(min, 4)} to ${sol(max, 4)}`} preserveAspectRatio="none">
      {ps.length > 1 ? <path d={ps.map((p, i) => `${i ? 'L' : 'M'}${x(i)},${y(p)}`).join(' ')} /> : null}
      {ps.map((p, i) => <circle key={i} cx={x(i)} cy={y(p)} r={3} />)}
    </svg>
  );
}

export default async function MarketItemPage({ params }: { params: Promise<{ itemMint: string }> }) {
  const { itemMint } = await params;
  const r = await read<MarketItem | null>(`/v1/market/items/${itemMint}`);
  if (!r.ok) return <><Head eyebrow="Marketplace" title={short(itemMint, 6)} /><Panel title="Item"><ReadFailed what="this item" error={r.error} /></Panel></>;
  if (!r.data) return <><Head eyebrow="Marketplace" title={short(itemMint, 6)} /><Panel title="Item"><Empty title="No item with this mint" what="The address is not an item mint on this cluster, or the item was burned in a forge." /></Panel></>;
  const it = r.data;
  const t = TEMPLATES.find((x) => x.id === it.data.templateId);
  const sales = it.sales;
  const last = sales[sales.length - 1];
  return (
    <>
      <Head eyebrow={`Marketplace · ${t?.name ?? `template ${it.data.templateId}`} · level ${it.data.level}`} title={itemSentence(it.data.templateId, it.data.params)}
        lede={`Royalty ${it.data.royaltyBps / 100}% of what it collects, paid to whoever holds it.`} right={<Link className="btn sm" href={`/armory/items/${it.item}`}>Armory record</Link>} />
      <div className="grid cols-main">
        <Panel title="Price history" meta={sales.length ? `${sales.length} ${sales.length === 1 ? 'sale' : 'sales'}` : 'sales only'}>
          {sales.length === 0 ? <Empty title="Never sold" what="Price history is drawn from completed sales only. No listing price, offer or estimate is shown as a price." /> : (
            <>
              <PriceLine sales={sales} />
              <table><thead><tr><th>When</th><th>Seller</th><th>Buyer</th><th className="r">Price</th><th className="r">Author share</th></tr></thead>
                <tbody>{[...sales].reverse().map((s) => <tr key={s.signature}><td className="faint"><a href={`https://solscan.io/tx/${s.signature}?cluster=devnet`} target="_blank" rel="noreferrer">{when(s.ts)}</a></td><td>{short(s.seller)}</td><td>{short(s.buyer)}</td><td className="r">{sol(s.price, 4)}</td><td className="r">{sol(s.resale, 4)}</td></tr>)}</tbody></table>
            </>
          )}
        </Panel>
        <Panel title="Listing" meta={it.listing ? 'for sale' : 'not listed'}>
          {it.listing ? (
            <>
              <dl className="kv"><dt>Price</dt><dd>{sol(it.listing.priceLamports, 4)}</dd><dt>Seller</dt><dd>{short(it.listing.seller, 6)}</dd><dt>Expires</dt><dd>{Number(it.listing.expiresAt) ? when(it.listing.expiresAt) : 'no expiry'}</dd><dt>Last sale</dt><dd>{last ? sol(last.price, 4) : 'never sold'}</dd></dl>
              <div style={{ height: 12 }} />
              <Actions>
                <Action route="market/buy" title="Buy" cta={`Buy for ${sol(it.listing.priceLamports, 4)}`} what="The price is checked again on chain; a changed price fails the buy." fixed={{ itemMint, maxPrice: it.listing.priceLamports }} />
                <Action route="market/delist" title="Delist" what="Only the seller can delist." fixed={{ itemMint }} />
              </Actions>
            </>
          ) : <Action route="market/list" title="List this item" cta="List" what="Expiry is a unix time in seconds; 0 lists with no expiry." fixed={{ itemMint }} fields={[{ name: 'priceLamports', label: 'Price', kind: 'sol' }, { name: 'expiresAt', label: 'Expires at', kind: 'int', hint: '0 for none' }]} />}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <div className="grid cols-2">
        <Panel title="Lineage" meta="from forge events">
          {it.lineage.parents.length === 0 && it.lineage.children.length === 0 ? <Empty title="No forges in its line" what="An item forged from two others shows them here, and so on back; forges that burned this item show below." /> : (
            <>
              <Tree node={it.lineage} />
              {it.lineage.children.length ? <p className="muted" style={{ fontSize: 13 }}>Burned to forge: {it.lineage.children.map((c) => <Link key={c} href={`/armory/items/${c}`}>{short(c)} </Link>)}</p> : null}
            </>
          )}
        </Panel>
        <Panel title="Rental" meta={it.lease ? at(LEASE_STATE, it.lease.state) : 'none'}>
          {it.lease ? (
            <>
              <dl className="kv"><dt>Token</dt><dd><Link href={`/t/${it.lease.tokenMint}`}>{short(it.lease.tokenMint, 6)}</Link>, slot {it.lease.slot}</dd><dt>Fee</dt><dd>{sol(it.lease.feeLamports, 4)}</dd><dt>Rent</dt><dd>{it.lease.rentBps / 100}% of what it collects</dd><dt>Term</dt><dd>{it.lease.state === 1 ? `until ${when(it.lease.endsAt)}` : `${Math.round(it.lease.termSecs / 3600)} h from acceptance`}</dd></dl>
              <div style={{ height: 12 }} />
              <Actions>
                {it.lease.state === 0 ? <Action route="market/lease/accept" title="Accept and pay the fee" fixed={{ itemMint }} /> : null}
                {it.lease.state === 0 ? <Action route="market/lease/withdraw" title="Withdraw the offer" what="Only the lessor." fixed={{ itemMint }} /> : null}
                {it.lease.state === 1 ? <Action route="market/lease/end" title="End the lease" what="After the term, anyone may end it; the item returns to the lessor." fixed={{ itemMint }} /> : null}
              </Actions>
            </>
          ) : <Action route="market/lease/offer" title="Offer for rent" fixed={{ itemMint }} fields={[
            { name: 'tokenMint', label: 'Token', kind: 'key' }, { name: 'slot', label: 'Slot', kind: 'int' }, { name: 'rentBps', label: 'Rent (basis points of what it collects)', kind: 'int' },
            { name: 'feeLamports', label: 'One-time fee', kind: 'sol' }, { name: 'termSecs', label: 'Term (seconds)', kind: 'int' },
          ]} />}
        </Panel>
      </div>
    </>
  );
}
